//! # websocket 渠道发送器
//!
//! 把投递任务整条推给连在本实例上的大屏：渲染好的标题正文、发布方原始上报的
//! `labels` / `annotations`（坐标、楼层这些落点要用的字段）、消息类别与来源、
//! 各级投递 ID 都在一帧里，大屏收到就能渲染并在地图上定位，不必再回查接口。
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
            data: task,
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
    /// 投递任务本身
    ///
    /// 直接引用 [`DeliveryMsg`]，任务加字段时帧自动跟上，不会再出现
    /// 「只推了标题正文、业务字段漏掉」的情况。
    data: &'a DeliveryMsg,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 一帧里的业务字段一个都不能少：大屏靠 `annotations` 落点、靠类别分组
    #[test]
    fn push_carries_whole_task() {
        let task = DeliveryMsg {
            persisted: false,
            delivery_id: None,
            delivery_target_id: None,
            delivery_channel_id: None,
            event_code: "position.updated".to_string(),
            message_id: 7,
            message_name: "定位更新".to_string(),
            category_code: "telemetry".to_string(),
            category_name: "遥测".to_string(),
            source_code: "dtw".to_string(),
            source_name: "数字孪生".to_string(),
            channel_id: 2,
            channel_code: "websocket".to_string(),
            target_id: 8888,
            business_id: None,
            business_trigger_ms: None,
            labels: HashMap::from([("deviceId".to_string(), "L1".to_string())]),
            annotations: HashMap::from([("lon".to_string(), "113.9".to_string())]),
            title: "张三 进入 1F".to_string(),
            content: "1F 东侧".to_string(),
            created_ms: 1_700_000_000_000,
        };

        let text = serde_json::to_string(&WsPush {
            kind: "message",
            data: &task,
        })
        .expect("帧序列化失败");
        let frame: serde_json::Value = serde_json::from_str(&text).expect("帧反序列化失败");

        let data = &frame["data"];
        assert_eq!(frame["type"], "message");
        assert_eq!(data["title"], "张三 进入 1F");
        assert_eq!(data["labels"]["deviceId"], "L1");
        assert_eq!(data["annotations"]["lon"], "113.9");
        assert_eq!(data["categoryCode"], "telemetry");
        assert_eq!(data["sourceCode"], "dtw");
        assert_eq!(data["targetId"], 8888);
        assert_eq!(data["persisted"], false);
    }
}
