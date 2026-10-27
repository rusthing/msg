//! # 投递执行器（wkr）
//!
//! `msg-svr` 在自己的进程里跑投递执行：一条消息渲染完、记账完，就按
//! 「目标 × 渠道」拆成投递任务发到该渠道的投递队列
//! （`msg_delivery_queue.code`），本模块订阅这些主题并把消息真正发出去。
//!
//! ## 为什么和执行在一个进程
//!
//! websocket 的连接在哪个进程、消息就必须在哪个进程发。执行器与 `msg-svr`
//! 同进程时，连接注册表、渠道配置、状态回写都是本地操作，不用引入额外的
//! 服务发现与结果回传链路。以后要拆成独立 worker，订阅与发送这两层可以
//! 原样搬走（主题契约不变），只需把回写改成 HTTP/NATS。
//!
//! ## 由谁驱动
//!
//! - 连接：`/msg/ws`（[`ws_handler`]），大屏带 `targetCategory`（目标类别编码）
//!   连上来，按编码登记；
//! - 投递：按 `msg_delivery_queue` 订阅（[`sub`]），websocket 是 Core NATS
//!   广播——每个实例都收到，各自查本地连接表；
//! - 发送：按渠道编码分发到 [`channel::ChannelSender`]；
//! - 回写：持久化消息才写状态（[`deliver_result`]），非持久化的发完就结束。
//!
//! ## 关停
//!
//! WS 长连接不会自己结束，而框架的优雅关停会等所有连接完成。所以：
//!
//! - 配置热更新重建订阅前，先 [`conn::close_all`]；
//! - 进程收到 SIGTERM/SIGINT 时同样先断开连接（[`watch_terminate_signal`]），
//!   否则 `stop_web_service` 会一直挂住。

pub mod channel;
pub mod conn;
pub(crate) mod deliver_result;
mod sub;
mod ws_handler;

use crate::config::get_msg_config;
use axum::Router;
use axum::routing::get;
use linkme::distributed_slice;
use msg_api::cst::MSG_DELIVERY_QUEUE_REFRESH_SUBJECT;
use msg_api::msg::DeliveryMsg;
use robotech::mq::nats::{self, NatsError};
use robotech::refresh;
use robotech::web::ROUTER_SLICE;
use std::sync::OnceLock;
use tracing::{error, info, warn};

/// # 注册 websocket 路由
///
/// 复用框架的 Web 服务器（TLS / 健康检查 / 优雅重启），只额外挂一个长连接端点。
/// 路由在服务器启动时构建，改 `msg.ws.path` 需要重启才生效。
#[distributed_slice(ROUTER_SLICE)]
static BUILD_WKR_ROUTER: fn() -> Router = build_wkr_router;

fn build_wkr_router() -> Router {
    let path = get_msg_config().ws.path.clone();
    info!("注册大屏长连接路由: {path}");
    Router::new().route(&path, get(ws_handler::upgrade))
}

/// # 装配投递执行器
///
/// 幂等：配置热更新会重复调用，每次先断开旧连接，再按最新的投递队列同步订阅。
/// 刷新信号订阅由框架按主题去重，重复注册只会复用已有订阅。
pub async fn setup_wkr() -> anyhow::Result<()> {
    // 长连接先断：旧连接上挂的是旧订阅发出来的消息，重建后由新连接承担
    conn::close_all();

    channel::init_senders();
    watch_terminate_signal();

    // 投递队列变更：各节点重新查库、换快照并增删订阅
    refresh::subscribe(&MSG_DELIVERY_QUEUE_REFRESH_SUBJECT, || async {
        sub::reload_delivery_queues().await
    })
    .await?;

    // 首次同步：订阅必须先于业务队列就绪，否则刚进来的消息直接发不出去
    sub::reload_delivery_queues().await?;

    Ok(())
}

/// # 投递队列写操作后发布刷新信号
///
/// 用于 `#[svc(after_write = ...)]`：队列配置写成功后通知所有节点重建订阅。
pub async fn on_delivery_queue_refresh() {
    if let Err(e) = refresh::publish(&MSG_DELIVERY_QUEUE_REFRESH_SUBJECT).await {
        error!("发布投递队列刷新信号失败: {e}");
    }
}

/// # 渠道写操作后：消息缓存与投递队列一起刷新
///
/// 渠道既是「消息发不发」的开关（`msg_channel.enabled`，消息缓存只加载启用的
/// 渠道），也是「有没有人订阅」的依据（投递队列的主题末段就是渠道编码）。
/// 所以渠道的写操作必须同时通知两处，只发消息缓存信号会留下一段时间里
/// 「不给它生成任务、却还挂着订阅」的错位状态。
///
/// 用于 [`crate::svc::MsgChannelSvc`] 的 `after_write`。
pub async fn on_channel_refresh() {
    crate::cache::on_message_cache_refresh().await;
    on_delivery_queue_refresh().await;
}

/// # 发布一条投递任务
///
/// 主题与是否持久化都取自该渠道的投递队列配置：
///
/// - `persisted = false` → Core NATS，at-most-once（websocket 就靠它做广播）
/// - `persisted = true` → JetStream，等服务器 ACK
///
/// 渠道没配投递队列时直接报错：没有队列就没有订阅方，发出去只会蒸发。
/// 发布失败（NATS 断连）由调用方决定怎么记账，这里只透传错误。
pub async fn publish_task(task: &DeliveryMsg) -> Result<(), NatsError> {
    let Some(queue) = sub::get_delivery_queue(&task.channel_code) else {
        return Err(NatsError::Publish(format!(
            "渠道 {} 没有可用的投递队列（未配置或已禁用），任务无处投递",
            task.channel_code
        )));
    };

    nats::publish(&queue.subject, task, queue.persisted).await?;
    Ok(())
}

/// # 进程退出时先断开长连接
///
/// 框架的 `shutdown_app` 会调用 `stop_web_service`，它等待所有在途连接结束；
/// WS 连接不会自己结束，只能在这里主动发 Close。框架没有暴露关停回调，
/// 所以自己监听一次 SIGTERM / SIGINT（与框架的信号处理并存）。
fn watch_terminate_signal() {
    static WATCHING: OnceLock<()> = OnceLock::new();
    if WATCHING.set(()).is_err() {
        return;
    }

    #[cfg(unix)]
    tokio::spawn(async {
        use tokio::signal::unix::{SignalKind, signal};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(e) => {
                warn!("监听 SIGTERM 失败，退出时可能无法及时断开长连接: {e}");
                return;
            }
        };
        let mut interrupt = match signal(SignalKind::interrupt()) {
            Ok(signal) => signal,
            Err(e) => {
                warn!("监听 SIGINT 失败，退出时可能无法及时断开长连接: {e}");
                return;
            }
        };

        tokio::select! {
            _ = terminate.recv() => info!("收到 SIGTERM，断开所有大屏连接"),
            _ = interrupt.recv() => info!("收到 SIGINT，断开所有大屏连接"),
        }
        conn::close_all();
    });

    #[cfg(not(unix))]
    warn!("当前平台不支持信号监听，退出时可能无法及时断开长连接");
}
