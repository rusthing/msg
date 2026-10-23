//! # 投递执行器（wkr）
//!
//! `msg-svr` 在自己的进程里跑投递执行：一条消息渲染完、记账完，就按
//! 「目标 × 渠道」拆成投递任务发到 `msg.delivery.{渠道编码}`，
//! 本模块订阅这些主题并把消息真正发出去。
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
//! - 连接：`/msg/ws`（[`ws_handler`]），大屏带 `targetId` 连上来；
//! - 投递：订阅 `msg.delivery.websocket`（[`sub`]），Core NATS 广播，
//!   每个实例都收到，各自查本地连接表；
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
use arc_swap::ArcSwapOption;
use axum::Router;
use axum::routing::get;
use linkme::distributed_slice;
use msg_api::cst::delivery_subject;
use msg_api::msg::DeliveryMsg;
use robotech::mq::nats::{self, NatsError};
use robotech::web::ROUTER_SLICE;
use std::sync::OnceLock;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// 当前投递任务订阅句柄
static DELIVERY_SUBSCRIPTION: ArcSwapOption<JoinHandle<()>> = ArcSwapOption::const_empty();

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
/// 幂等：配置热更新会重复调用，每次先断开旧连接、停掉旧订阅，再重建。
pub async fn setup_wkr() -> anyhow::Result<()> {
    // 长连接先断：旧连接上挂的是旧订阅发出来的消息，重建后由新连接承担
    conn::close_all();
    if let Some(old) = DELIVERY_SUBSCRIPTION.swap(None) {
        old.abort();
    }

    channel::init_senders();
    watch_terminate_signal();

    let handle = sub::start_delivery_subscription().await?;
    DELIVERY_SUBSCRIPTION.store(Some(handle));

    Ok(())
}

/// # 发布一条投递任务
///
/// 走 Core NATS：websocket 需要「所有实例都收到、各自查本地连接」的广播语义，
/// 不需要 JetStream 的持久化与应答（大屏离线就是这次没送达，不补投）。
///
/// 发布失败（NATS 断连）时由调用方决定怎么记账，这里只透传错误。
pub async fn publish_task(task: &DeliveryMsg) -> Result<(), NatsError> {
    let subject = delivery_subject(&task.channel_code);
    nats::publish(&subject, task, false).await?;
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
