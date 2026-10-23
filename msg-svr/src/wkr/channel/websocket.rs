//! # websocket 渠道发送器
//!
//! 把投递任务推给连在本实例上的大屏。推送内容就是投递任务本身：标题与正文
//! 已经由模板渲染好，`labels` / `annotations` 是渲染的输入，不随投递下发。
//!
//! 目标不在本实例（或压根没连）时返回 [`SendOutcome::NoConnection`]：
//! websocket 是 at-most-once，没有连接就没有必要重试，也不需要补投——
//! 大屏自己刷新列表即可。

use super::{ChannelSender, SendOutcome};
use crate::wkr::conn;
use async_trait::async_trait;
use msg_api::cst::MSG_CHANNEL_CODE_WEBSOCKET;
use msg_api::msg::DeliveryMsg;
use serde::Serialize;
use tracing::warn;

/// websocket 渠道发送器
pub struct WebSocketSender;

#[async_trait]
impl ChannelSender for WebSocketSender {
    fn code(&self) -> &'static str {
        MSG_CHANNEL_CODE_WEBSOCKET
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let push = WsPush {
            kind: "message",
            data: WsPushData {
                delivery_id: task.delivery_id,
                event_code: &task.event_code,
                business_id: task.business_id,
                title: &task.title,
                content: &task.content,
                created_ms: task.created_ms,
            },
        };

        let text = match serde_json::to_string(&push) {
            Ok(text) => text,
            Err(e) => return SendOutcome::Failed(format!("推送内容序列化失败: {e}")),
        };

        if conn::send(task.target_id, text) {
            // 地址只用于排查：哪个目标、连在哪个实例上
            SendOutcome::Delivered {
                address: format!("target:{}", task.target_id),
            }
        } else {
            warn!(
                "目标未连接，消息丢弃: target_id={}, event_code={}",
                task.target_id, task.event_code
            );
            SendOutcome::NoConnection
        }
    }
}

/// 推给大屏的一帧
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPush<'a> {
    /// 帧类型，目前只有 `message`
    #[serde(rename = "type")]
    kind: &'static str,
    data: WsPushData<'a>,
}

/// 推给大屏的消息体
///
/// 只有渲染结果，没有 `labels` / `annotations`：那两份是模板的输入，
/// 投递的产物就是这里的标题与正文。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPushData<'a> {
    /// 投递记录 ID（非持久化消息没有）
    delivery_id: Option<u64>,
    event_code: &'a str,
    /// 业务 ID（非持久化消息没有）
    business_id: Option<u64>,
    title: &'a str,
    content: &'a str,
    created_ms: u64,
}
