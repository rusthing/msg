//! # 投递任务订阅
//!
//! 订阅 `msg.delivery.websocket`，把任务交给渠道发送器。
//!
//! ## 为什么是 Core NATS
//!
//! websocket 连接只存在于某一个实例上：任务必须让**所有实例都收到**，
//! 各自查本地连接表，谁有连接谁发。用 JetStream + 共享队列组会变成
//! 「A 实例收到任务，大屏却连在 B 实例上」，消息直接丢。
//!
//! 不用 JetStream 也就没有 ACK/重投/死信：websocket 本来就是 at-most-once，
//! 目标不在线就是这次没送达，不需要补投（大屏刷新列表即可）。

use crate::wkr::{channel, deliver_result};
use msg_api::cst::{MSG_CHANNEL_CODE_WEBSOCKET, delivery_subject};
use msg_api::msg::DeliveryMsg;
use robotech::mq::nats::{self, NatsError};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

/// # 启动 websocket 渠道的投递订阅
pub async fn start_delivery_subscription() -> anyhow::Result<Arc<JoinHandle<()>>> {
    let subject = delivery_subject(MSG_CHANNEL_CODE_WEBSOCKET);

    let handle = nats::subscribe(&subject, None, None, None, move |msg| async move {
        let task: DeliveryMsg = match serde_json::from_slice(&msg.payload) {
            Ok(task) => task,
            Err(e) => {
                // Core NATS 没有重投，解析失败只能丢
                error!("投递任务解析失败，丢弃: {e}");
                return Ok::<(), NatsError>(());
            }
        };

        let outcome = channel::dispatch(&task).await;

        if task.persisted {
            deliver_result::record(&task, &outcome).await;
        } else {
            // 非持久化：不落库、不回写，成败都到此为止
            debug!(
                "非持久化消息已投递: event_code={}, target_id={}, outcome={}",
                task.event_code,
                task.target_id,
                outcome.detail()
            );
        }

        Ok::<(), NatsError>(())
    })
    .await?;

    info!("已订阅投递任务: {subject}");
    Ok(handle)
}
