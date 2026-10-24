//! # websocket 渠道发送器
//!
//! 推给大屏的一帧只有三个字段——事件编码、标题、内容：
//!
//! ```json
//! {"eventCode":"…","title":"…","content":"…"}
//! ```
//!
//! 大屏收到就能渲染，不必再回查接口。落点要用的坐标不在独立字段里：它们由内容
//! 模板渲染进 `content`，大屏按约定的格式解析 `content` 即可。发布方上报的
//! `labels` / `annotations` 只是模板的输入，不随这一帧下发。
//!
//! 投递三张表的行 ID 与目标/渠道字段也不下发：大屏是只收不发的长连接
//! （见 [`crate::wkr::ws_handler`]），拿不到回执，这些字段没有任何消费方。
//!
//! 目标类别下没有连接（或压根没连）时返回 [`SendOutcome::NoConnection`]：
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
            event_code: &task.event_code,
            title: &task.title,
            content: &task.content,
        };

        let text = match serde_json::to_string(&push) {
            Ok(text) => text,
            Err(e) => return SendOutcome::Failed(format!("推送内容序列化失败: {e}")),
        };

        // 按目标类别编码找连接：大屏连上来时上报的就是这个编码，
        // 本实例把这个类别下的连接全发一遍，别的实例收到同一条任务也会各自发
        if conn::send(&task.target_category_code, text) {
            // 地址只用于排查：这条投递落在哪个类别的连接上
            SendOutcome::Delivered {
                address: format!("category:{}", task.target_category_code),
            }
        } else {
            // 「本实例没有连接」不是投递失败：屏可能连在别的实例上，
            // 那边会把这条渠道记成送达
            warn!(
                "本实例没有该类别下的连接，消息丢弃: category={}, event_code={}",
                task.target_category_code, task.event_code
            );
            SendOutcome::NoConnection
        }
    }
}

/// 推给大屏的一帧
///
/// 只放渲染结果，不放投递元数据：字段越少，大屏侧解析越稳，帧也越小。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPush<'a> {
    /// 事件编码（`msg_delivery.event_code`），大屏按它决定怎么渲染
    event_code: &'a str,
    /// 渲染好的标题
    title: &'a str,
    /// 渲染好的内容；定位消息的落点就在这段 JSON 文本里
    content: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> DeliveryMsg {
        DeliveryMsg {
            persisted: false,
            delivery_id: None,
            delivery_target_id: None,
            delivery_channel_id: None,
            event_code: "position.updated".to_string(),
            message_id: 7,
            event_source_id: 3,
            message_category_id: 5,
            business_id: None,
            business_trigger_ms: None,
            title: "张三 进入 1F".to_string(),
            // 定位消息的落点就在内容里，大屏靠它在地图上定位
            content: r#"{"text":"1F 东侧","lon":113.9,"lat":22.5}"#.to_string(),
            ext: Some("extra".to_string()),
            create_ms: 1_700_000_000_000,
            target_category_code: "screen".to_string(),
            target_category_id: 11,
            // 空串 = 该类别下的全部目标（按类别广播）
            target_id: String::new(),
            channel_id: 2,
            channel_code: "websocket".to_string(),
        }
    }

    /// 一帧只有三个字段：事件编码、标题、内容
    #[test]
    fn frame_carries_render_result() {
        let task = task();

        let text = serde_json::to_string(&WsPush {
            event_code: &task.event_code,
            title: &task.title,
            content: &task.content,
        })
        .expect("帧序列化失败");
        let frame: serde_json::Value = serde_json::from_str(&text).expect("帧反序列化失败");

        assert_eq!(frame["eventCode"], "position.updated");
        assert_eq!(frame["title"], "张三 进入 1F");
        // 内容是 JSON 文本，原样透传，不在推送侧二次解析
        let content: serde_json::Value =
            serde_json::from_str(frame["content"].as_str().expect("content 不是字符串"))
                .expect("content 不是合法 JSON");
        assert_eq!(content["lon"], 113.9);
    }

    /// 投递元数据不下发：任务里有 ext、行 ID、目标/渠道字段，帧里一个都不能出现
    #[test]
    fn delivery_metadata_is_not_pushed() {
        let task = task();

        let text = serde_json::to_string(&WsPush {
            event_code: &task.event_code,
            title: &task.title,
            content: &task.content,
        })
        .expect("帧序列化失败");
        let frame: serde_json::Value = serde_json::from_str(&text).expect("帧反序列化失败");

        let mut keys: Vec<&str> = frame
            .as_object()
            .expect("帧不是对象")
            .keys()
            .map(|key| key.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["content", "eventCode", "title"]);
    }
}
