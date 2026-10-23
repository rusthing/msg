//! # 投递任务订阅
//!
//! 按 `msg_delivery_queue` 的配置订阅投递主题，把任务交给渠道发送器。
//!
//! ## 订阅什么由表决定
//!
//! 一个渠道对应一条投递队列（`msg_delivery_queue.channel_id` 唯一）：
//!
//! - `code` 是投递主题，发布与订阅共用这一份，换主题不用改代码；
//! - `persisted` 决定订阅方式：`false` 走 Core NATS 广播，`true` 走
//!   JetStream Push + 队列组，发送失败 NAK 回服务端按 `max_deliver` 重投。
//!
//! ## 为什么 websocket 必须是广播
//!
//! websocket 连接只存在于某一个实例上：任务必须让**所有实例都收到**，
//! 各自查本地连接表，谁有连接谁发。用 JetStream + 共享队列组会变成
//! 「A 实例收到任务，大屏却连在 B 实例上」，消息直接丢。短信 / 邮箱这类
//! 没有本地连接的渠道才适合持久化 + 负载均衡。
//!
//! ## 队列快照为什么留在本模块
//!
//! 发布侧（[`crate::wkr::publish_task`]）要知道「这个渠道发到哪个 subject、
//! 要不要等 ACK」，所以队列快照必须常驻内存——这点和消息入口队列不同
//! （那边只有订阅方，收到刷新信号直接查库即可，见 [`crate::sub`]）。
//! 快照与订阅状态一起维护在这里：
//!
//! ```text
//! 队列写操作 → on_delivery_queue_refresh 发布 refresh.msg.delivery-queue
//!   → 各节点 reload_delivery_queues：查库 → 换快照 → 增删订阅
//! ```

use crate::config::get_msg_config;
use crate::dto::{MsgChannelQueryDto, MsgDeliveryQueueQueryDto};
use crate::sub::{QUEUE_GROUP, nats_name_token};
use crate::svc::{MsgChannelSvc, MsgDeliveryQueueSvc};
use crate::wkr::channel::SendOutcome;
use crate::wkr::{channel, deliver_result};
use arc_swap::ArcSwapOption;
use msg_api::msg::DeliveryMsg;
use robotech::db::get_db_conn;
use robotech::mq::nats::{self, NatsError};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

/// # 一个渠道的投递队列
///
/// 发布侧与订阅侧共用这份快照：发布侧按 `channel_code` 查到它取 `subject` /
/// `persisted`，订阅侧按 `subject` 建立（或复用自己的）订阅。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryQueue {
    /// 渠道编码（`msg_channel.code`），任务按它认领发送器
    pub channel_code: String,
    /// 投递主题（`msg_delivery_queue.code`）
    pub subject: String,
    /// 是否持久化（`true` → JetStream，`false` → Core NATS 广播）
    pub persisted: bool,
}

/// # 渠道编码 → 投递队列
///
/// 发布侧每次投递都要查，所以放内存；变更由刷新信号驱动重载。
static DELIVERY_QUEUES: ArcSwapOption<HashMap<String, DeliveryQueue>> =
    ArcSwapOption::const_empty();

/// # 投递订阅句柄
///
/// key = `subject`（一条队列一个 subject，`code` 上有唯一约束）。句柄还记着
/// 建立订阅时用的方式与重投上限：配置改了就重建——`get_or_create_consumer`
/// 对已存在的 consumer 不会应用新配置，只有重新 `subscribe` 才会走到
/// `update_consumer`。
#[derive(Debug)]
struct DeliverySubscription {
    handle: Arc<JoinHandle<()>>,
    /// 是否 JetStream 订阅（Core NATS 没有 consumer，重投上限对它无意义）
    persisted: bool,
    /// 建立订阅时应用的重投上限
    max_deliver: i64,
    /// 该订阅属于哪个渠道，只用于日志排查
    channel_code: String,
}

/// # 当前投递订阅
static DELIVERY_SUBSCRIBERS: ArcSwapOption<HashMap<String, DeliverySubscription>> =
    ArcSwapOption::const_empty();

/// # 读取某渠道的投递队列
///
/// 发布侧用：返回 `None` 表示这个渠道没配投递队列，任务无处投递。
pub fn get_delivery_queue(channel_code: &str) -> Option<DeliveryQueue> {
    DELIVERY_QUEUES
        .load_full()
        .and_then(|queues| queues.get(channel_code).cloned())
}

/// # 重新加载投递队列
///
/// 启动时调用一次，之后每次收到 `refresh.msg.delivery-queue` 都调用。
/// 三步：查库组装快照 → 换快照（发布侧立刻按新配置走）→ 同步订阅。
///
/// 快照先于订阅替换：反过来的话，新订阅建立前发布侧还按旧主题发，
/// 中间那段消息会落到没人订阅的主题上。
pub async fn reload_delivery_queues() -> anyhow::Result<()> {
    let db = get_db_conn()?;

    // ── 渠道 ID → 编码：投递队列只存 channel_id，发布侧按编码认领 ──

    let channels: HashMap<u64, String> =
        MsgChannelSvc::list_by_query_dto(MsgChannelQueryDto::default(), Some(db.as_ref()))
            .await?
            .extra
            .unwrap_or_default()
            .into_iter()
            .map(|channel| (channel.id.0, channel.code))
            .collect();

    let rows = MsgDeliveryQueueSvc::list_by_query_dto(
        MsgDeliveryQueueQueryDto::default(),
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();

    let queues: HashMap<String, DeliveryQueue> = rows
        .into_iter()
        .filter_map(|queue| {
            let Some(channel_code) = channels.get(&queue.channel_id.0).cloned() else {
                // 外键是 restrict，正常删不出这种数据；真出现了就别订阅，
                // 也别让发布侧按它发——查不到的渠道没有发送器
                warn!(
                    "投递队列 {} 指向不存在的渠道 {}，跳过",
                    queue.code, queue.channel_id.0
                );
                return None;
            };
            Some((
                channel_code.clone(),
                DeliveryQueue {
                    channel_code,
                    subject: queue.code,
                    persisted: queue.persisted,
                },
            ))
        })
        .collect();

    let count = queues.len();
    DELIVERY_QUEUES.store(Some(Arc::new(queues.clone())));

    let max_deliver = get_msg_config().effective_max_deliver();
    sync_delivery_subscriptions(&queues, max_deliver).await;

    info!("投递队列同步完成: {count} 个渠道");
    Ok(())
}

/// # 同步投递订阅
///
/// - 队列还在、订阅任务没退出、方式与重投上限都没变 → 保留
/// - 订阅任务已退出（如 NATS 连接被换成新的 Client）→ 重建
/// - 方式 / 重投上限变了 → 停掉旧的，按新配置重建
/// - 队列已从表里删掉 → 停止订阅
async fn sync_delivery_subscriptions(queues: &HashMap<String, DeliveryQueue>, max_deliver: i64) {
    let current = DELIVERY_SUBSCRIBERS
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()));

    let mut next: HashMap<String, DeliverySubscription> = HashMap::new();

    for queue in queues.values() {
        if let Some(subscription) = current.get(&queue.subject) {
            let reusable = !subscription.handle.is_finished()
                && subscription.persisted == queue.persisted
                && (!queue.persisted || subscription.max_deliver == max_deliver);

            if reusable {
                next.insert(
                    queue.subject.clone(),
                    DeliverySubscription {
                        handle: Arc::clone(&subscription.handle),
                        persisted: subscription.persisted,
                        max_deliver: subscription.max_deliver,
                        channel_code: subscription.channel_code.clone(),
                    },
                );
                continue;
            }

            if subscription.handle.is_finished() {
                warn!("投递订阅任务已退出，将重建: {}", queue.subject);
            } else {
                info!("投递队列配置变更，重建订阅: {}", queue.subject);
                subscription.handle.abort();
            }
        }

        match start_delivery_subscription(queue, max_deliver).await {
            Ok(handle) => {
                info!(
                    "启动投递订阅成功: channel={}, subject={}, persisted={}",
                    queue.channel_code, queue.subject, queue.persisted
                );
                next.insert(
                    queue.subject.clone(),
                    DeliverySubscription {
                        handle,
                        persisted: queue.persisted,
                        max_deliver,
                        channel_code: queue.channel_code.clone(),
                    },
                );
            }
            Err(e) => error!("启动投递订阅失败 {}: {e}", queue.subject),
        }
    }

    // ── 停掉已移除的队列（含改了 subject 后剩下的旧主题）──

    for (subject, subscription) in current.iter() {
        if !next.contains_key(subject) {
            info!(
                "停止投递订阅: channel={}, subject={subject}",
                subscription.channel_code
            );
            subscription.handle.abort();
        }
    }

    DELIVERY_SUBSCRIBERS.store(Some(Arc::new(next)));
}

/// # 为一条投递队列建立订阅
///
/// - `persisted = false` → Core NATS 订阅，**不加队列组**：所有实例都要收到，
///   各自查本地连接表（websocket 的唯一正确语义）
/// - `persisted = true` → JetStream Push 订阅，加入 [`QUEUE_GROUP`] 由服务端
///   在副本间负载均衡；Stream / Consumer 名由 subject 派生（NATS 资源名不允许
///   `.`，不能直接拼 subject）
async fn start_delivery_subscription(
    queue: &DeliveryQueue,
    max_deliver: i64,
) -> Result<Arc<JoinHandle<()>>, NatsError> {
    use async_nats::jetstream::{consumer, stream};

    let subject = queue.subject.clone();
    let owned_subject = subject.clone();
    let transport_persisted = queue.persisted;

    let handler = move |msg: async_nats::Message| {
        let subject = owned_subject.clone();
        async move {
            let task: DeliveryMsg = match serde_json::from_slice(&msg.payload) {
                Ok(task) => task,
                Err(e) => {
                    // 解析不出来就没人能处理它，重投多少次都一样
                    error!("投递任务解析失败，丢弃 [{subject}]: {e}");
                    return Ok::<(), NatsError>(());
                }
            };

            let outcome = channel::dispatch(&task).await;

            if task.persisted {
                deliver_result::record(&task, &outcome).await;
            } else {
                // 非持久化：不落库、不回写，成败都到此为止
                debug!(
                    "非持久化消息已投递: event_code={}, category={}, target_id={}, outcome={}",
                    task.event_code,
                    task.target_category_code,
                    task.target_id,
                    outcome.detail()
                );
            }

            // JetStream 投递队列：发送失败要 NAK 回服务端，由它按 `max_deliver`
            // 重投，超限后进死信（`dlq.{subject}`）。Core NATS 没有 ACK/NAK，
            // 返回值只影响日志。
            if transport_persisted && let SendOutcome::Failed(reason) = &outcome {
                return Err(NatsError::Handle(format!("投递失败 [{subject}]: {reason}")));
            }

            Ok::<(), NatsError>(())
        }
    };

    if queue.persisted {
        let name_token = nats_name_token(&subject);
        let stream_config = stream::Config {
            name: format!("MSG-{name_token}"),
            subjects: vec![subject.clone()],
            // 保留 7 天：消费端长时间停机后仍能补上，避免流无限增长
            max_age: Duration::from_secs(7 * 24 * 3600),
            ..Default::default()
        };
        let consumer_config = consumer::push::Config {
            durable_name: Some(format!("msg-svr-dlq-{name_token}")),
            deliver_policy: consumer::DeliverPolicy::All,
            ack_policy: consumer::AckPolicy::Explicit,
            max_deliver,
            ..Default::default()
        };

        nats::subscribe(
            &subject,
            Some(QUEUE_GROUP),
            Some(stream_config),
            Some(consumer_config),
            handler,
        )
        .await
    } else {
        nats::subscribe(&subject, None, None, None, handler).await
    }
}
