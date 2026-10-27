//! # 消息队列刷新装配
//!
//! 消息队列不进缓存（详见 [`crate::cache`]），订阅状态由 [`crate::sub`] 自己维护，
//! 因此队列变更走独立的 direct 链路：收到刷新信号后直接查库并同步 NATS 订阅，
//! 不产生缓存快照。
//!
//! ```text
//! 消息队列写操作 → on_message_queue_refresh() 发布信号
//!   → 各节点 setup_message_queue_refresh 注册的消费者
//!     → reload_message_queue_subscriptions() → 查库 → 增删 NATS 订阅
//! ```

use crate::sub::{ensure_normal_message_queue, reload_message_queue_subscriptions};
use msg_api::cst::MSG_MESSAGE_QUEUE_REFRESH_SUBJECT;
use robotech::refresh;

/// # 装配消息队列刷新链路
///
/// 与 [`crate::cache::setup_cache`] 平级，由 `main.rs` 在缓存就绪后调用。
///
/// ## 步骤
///
/// 1. 注册刷新信号消费者：任一节点写消息队列后，各节点直接查库同步订阅
/// 2. 普通消息队列保底：只在启动时补一次，保证 `msg.message.normal` 一定有订阅与 stream
/// 3. 首次同步订阅：按库内队列配置建立启动时的订阅集合
///
/// ## 错误
///
/// 刷新信号订阅失败或首次同步失败时返回错误，由 `main.rs` 终止启动。
pub async fn setup_message_queue_refresh() -> anyhow::Result<()> {
    // 队列订阅：收到刷新信号后直接查库并同步 NATS 订阅（不经过缓存）
    refresh::subscribe(&MSG_MESSAGE_QUEUE_REFRESH_SUBJECT, || async {
        reload_message_queue_subscriptions().await
    })
    .await?;

    // 普通消息队列保底：只在启动时补一次，保证 msg.message.normal 一定有订阅与 stream
    ensure_normal_message_queue().await?;
    reload_message_queue_subscriptions().await?;

    Ok(())
}
