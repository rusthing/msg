//! # websocket 渠道发送器
//!
//! 把投递任务整条推给连在本实例上的大屏：渲染好的标题、内容、附加内容，
//! 投递三张表的行 ID 与目标/渠道字段都在一帧里，大屏收到就能渲染，不必再回查接口。
//!
//! 落点要用的坐标不在独立字段里：它们由内容模板渲染进 `content`，大屏按约定的
//! 格式解析 `content` 即可。发布方上报的 `labels` / `annotations` 只是模板的
//! 输入，不随这一帧下发。
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
            kind: "message",
            data: task,
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
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPush<'a> {
    /// 帧类型，目前只有 `message`
    #[serde(rename = "type")]
    kind: &'static str,
    /// 投递任务本身
    ///
    /// 直接引用 [`DeliveryMsg`]，任务加字段时帧自动跟上，不会再出现
    /// 「只推了标题正文、业务字段漏掉」的情况。
    data: &'a DeliveryMsg,
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

    /// 一帧里该有的字段一个都不能少：大屏靠 `content` 落点，靠行 ID 回执
    #[test]
    fn push_carries_whole_task() {
        let task = task();

        let text = serde_json::to_string(&WsPush {
            kind: "message",
            data: &task,
        })
        .expect("帧序列化失败");
        let frame: serde_json::Value = serde_json::from_str(&text).expect("帧反序列化失败");

        let data = &frame["data"];
        assert_eq!(frame["type"], "message");
        assert_eq!(data["title"], "张三 进入 1F");
        // 内容是 JSON 文本，原样透传，不在推送侧二次解析
        let content: serde_json::Value =
            serde_json::from_str(data["content"].as_str().expect("content 不是字符串"))
                .expect("content 不是合法 JSON");
        assert_eq!(content["lon"], 113.9);
        assert_eq!(data["ext"], "extra");
        assert_eq!(data["eventSourceId"], 3);
        assert_eq!(data["messageCategoryId"], 5);
        // 大屏按类别编码认领：帧里必须有它，光有类别 ID 对不上连接
        assert_eq!(data["targetCategoryCode"], "screen");
        assert_eq!(data["targetCategoryId"], 11);
        assert_eq!(data["targetId"], "");
        assert_eq!(data["channelId"], 2);
        assert_eq!(data["createMs"], 1_700_000_000_000i64);
        assert_eq!(data["persisted"], false);

        // 模板输入不下发：这两份只在 msg_delivery 快照里
        assert!(data.get("labels").is_none());
        assert!(data.get("annotations").is_none());
    }

    /// 没有附加内容时整个字段不出现，消费方用 `is_some()` 判断才有效
    #[test]
    fn absent_ext_is_omitted_from_frame() {
        let task = DeliveryMsg {
            ext: None,
            ..task()
        };

        let text = serde_json::to_string(&WsPush {
            kind: "message",
            data: &task,
        })
        .expect("帧序列化失败");
        let frame: serde_json::Value = serde_json::from_str(&text).expect("帧反序列化失败");

        assert!(frame["data"].get("ext").is_none());
    }
}
