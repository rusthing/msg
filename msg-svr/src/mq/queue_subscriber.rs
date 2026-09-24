//! # 队列订阅管理
//!
//! 维护当前活跃的消息队列订阅句柄，在队列缓存刷新时根据最新的队列信息
//! 同步订阅状态：新增队列则启动新订阅，移除的队列则停止旧的订阅线程。
//!
//! ## 数据流
//!
//! ```text
//! refresh_msg_queue_cache
//!   → 从 DB 加载所有 msg_message_queue
//!   → 提取所有 CachedQueue
//!   → sync_queue_subscriptions(queues)
//!     → 对比旧订阅：新增 / 保留 / 移除
//!     → 调用 NATS subscribe 启动新线程
//!     → 停止不再需要的订阅线程
//! ```
//!
//! 注意：消息模板缓存（`refresh_msg_message_cache`）不会触发队列重新订阅。

use arc_swap::ArcSwapOption;
use robotech::mq::nats;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::cache::CachedQueue;
use crate::mq::process_message;

/// # 全局队列订阅句柄
///
/// key = `queue.code`（即 NATS subject），value = 订阅线程句柄。
/// 使用 [`ArcSwapOption`] 实现无锁热替换。
static QUEUE_SUBSCRIBERS: ArcSwapOption<HashMap<String, Arc<JoinHandle<()>>>> =
    ArcSwapOption::const_empty();

/// # 同步队列订阅
///
/// 根据给定的队列集合，对比当前活跃订阅：
/// - 队列 `code` 不存在于当前订阅 → 启动新订阅
/// - 队列 `code` 已存在 → 保留（跳过）
/// - 当前订阅中的 `code` 不在 `queues` 中 → 停止
///
/// 每个队列根据 `persisted` 字段自动选择 Core NATS（`false`）或
/// JetStream Push（`true`）订阅模式。
///
/// ## 参数
///
/// - `queues`: 从当前缓存中提取的所有唯一消息队列列表
pub async fn sync_queue_subscriptions(queues: &HashSet<CachedQueue>) {
    let current = QUEUE_SUBSCRIBERS
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()));

    let current_codes: HashSet<String> = current.keys().cloned().collect();
    let new_codes: HashSet<String> = queues.iter().map(|q| q.code.clone()).collect();

    // ── 停止已移除的队列订阅 ──

    let removed: HashSet<&String> = current_codes.difference(&new_codes).collect();
    let mut next = HashMap::new();

    for (code, handle) in current.iter() {
        if removed.contains(code) {
            info!("停止队列订阅: {code}");
            handle.abort();
        } else {
            next.insert(code.clone(), Arc::clone(handle));
        }
    }

    // ── 启动新增的队列订阅 ──

    let added: HashSet<&String> = new_codes.difference(&current_codes).collect();

    for &code in &added {
        // 默认不持久化（Core NATS）
        let persisted = queues
            .iter()
            .find(|q| q.code == *code)
            .map(|q| q.persisted)
            .unwrap_or(false);

        let handle = match start_subscription_for_queue(code, persisted).await {
            Ok(handle) => {
                info!("启动队列订阅成功: {code}");
                handle
            }
            Err(e) => {
                error!("启动队列订阅失败 {code}: {e}");
                continue;
            }
        };

        next.insert(code.clone(), handle);
    }

    if !added.is_empty() || !removed.is_empty() || QUEUE_SUBSCRIBERS.load_full().is_none() {
        QUEUE_SUBSCRIBERS.store(Some(Arc::new(next)));
    }
}

/// # 为指定队列启动 NATS 订阅
///
/// - `persisted = false` → Core NATS 订阅（at-most-once）
/// - `persisted = true` → JetStream Push 订阅，创建的 Stream 名称为 `MSG-{code}`，
///   Consumer 名称为 `msg-svr-{code}`
async fn start_subscription_for_queue(
    subject: &str,
    persisted: bool,
) -> Result<Arc<JoinHandle<()>>, Box<dyn std::error::Error + Send + Sync>> {
    use async_nats::jetstream::{consumer, stream};

    let owned_subject = subject.to_string();

    if persisted {
        let stream_name = format!("MSG-{subject}");
        let stream_config = stream::Config {
            name: stream_name.clone(),
            subjects: vec![subject.to_string()],
            ..Default::default()
        };

        let consumer_config = consumer::push::Config {
            durable_name: Some(format!("msg-svr-{subject}")),
            deliver_policy: consumer::DeliverPolicy::All,
            ack_policy: consumer::AckPolicy::Explicit,
            ..Default::default()
        };

        nats::subscribe(
            subject,
            None,
            Some(stream_config),
            Some(consumer_config),
            move |msg| {
                let subj = owned_subject.clone();
                async move {
                    info!(
                        "收到 JetStream 消息 [{}]: {}",
                        subj,
                        String::from_utf8_lossy(&msg.payload)
                    );
                    process_message(&subj, &msg.payload).await
                }
            },
        )
        .await
        .map_err(Into::into)
    } else {
        nats::subscribe(
            subject,
            None,
            None,
            None,
            move |msg| {
                let subj = owned_subject.clone();
                async move {
                    info!(
                        "收到 Core NATS 消息 [{}]: {}",
                        subj,
                        String::from_utf8_lossy(&msg.payload)
                    );
                    process_message(&subj, &msg.payload).await
                }
            },
        )
        .await
        .map_err(Into::into)
    }
}