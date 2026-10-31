//! # websocket 渠道发送器
//!
//! 推给大屏的一帧：
//!
//! ```json
//! {"eventCode":"…","title":"…","content":"…","ext":"…","firstReceiveMs":1761…,"businessTriggerMs":1761…}
//! ```
//!
//! 大屏收到就能渲染，不必再回查接口。落点要用的坐标不在独立字段里：它们由内容
//! 模板渲染进 `content`，大屏按约定的格式解析 `content` 即可。发布方上报的
//! `labels` / `annotations` 只是模板的输入，不随这一帧下发。
//!
//! ## 两个时间锚点
//!
//! `firstReceiveMs` 是链路第一跳收到原始事件的服务端时刻，`businessTriggerMs`
//! 是本次通知的业务时间（发布方时钟）。大屏是只收不发的长连接，没有回执通道，
//! 帧里不带时间戳，它就只能拿本地收帧时刻当起点，既算不出真实延迟，也判不了
//! 「这条报警是不是早就该到了」。两个值都可能缺（发布方没接入），缺了就是
//! `null`，大屏按「不可算」处理。
//!
//! ## `ext` 是给机器看的那一份
//!
//! `title` / `content` 是人看的文案，`ext` 是同一件事的结构化版本：报警 ID、
//! 定位坐标、Unity 场景坐标这类字段。它由发布方（或消息中心的 `ext_template`）
//! 渲染成一段文本，格式不做约定——报警中心给的是 JSON，大屏自己解析。
//!
//! 没有 `ext` 的消息整个字段都不出现（`skip_serializing_if`），大屏侧按
//! 「有没有 `ext`」判断这条消息带不带结构化数据，不需要区分 `null` 和空串。
//!
//! ## 不下发什么
//!
//! 投递三张表的行 ID 与目标/渠道字段不下发：大屏是只收不发的长连接
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
            ext: task.ext.as_deref(),
            first_receive_ms: task.first_receive_ms,
            business_trigger_ms: task.business_trigger_ms,
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
/// 只放渲染结果 + 两个时间锚点：字段越少，大屏侧解析越稳，帧也越小。
///
/// 时间锚点是大屏唯一能自己算延迟的地方：它拿不到库，也没有回执通道，
/// 想判断「这条报警从事件发生到现在过了多久」「该不该判过期不弹」，
/// 只能靠帧里带下来的时间戳。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPush<'a> {
    /// 事件编码（`msg_delivery.event_code`），大屏按它决定怎么渲染
    event_code: &'a str,
    /// 渲染好的标题
    title: &'a str,
    /// 渲染好的内容；定位消息的落点就在这段 JSON 文本里
    content: &'a str,
    /// 渲染好的附加内容（`msg_delivery.ext`），没有就不出现在帧里
    #[serde(skip_serializing_if = "Option::is_none")]
    ext: Option<&'a str>,
    /// 链路第一跳收到原始事件的时刻（毫秒，服务端时钟）
    ///
    /// 端到端延迟只用它算：它是链路上第一个可信的服务端时间，不受终端时钟
    /// 偏差、离线补传影响。发布方没带就是 `null`，大屏按「不可算」处理。
    #[serde(skip_serializing_if = "Option::is_none")]
    first_receive_ms: Option<u64>,
    /// 本次通知的业务时间（毫秒，发布方时钟）
    ///
    /// 与业务库的幂等键同值，大屏回查报警详情时用它对齐同一次通知；
    /// 不适合单独用来算延迟（时钟可能偏）。
    #[serde(skip_serializing_if = "Option::is_none")]
    business_trigger_ms: Option<u64>,
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
            first_trigger_ms: None,
            first_receive_ms: None,
            title: "张三 进入 1F".to_string(),
            // 定位消息的落点就在内容里，大屏靠它在地图上定位
            content: r#"{"text":"1F 东侧","lon":113.9,"lat":22.5}"#.to_string(),
            ext: None,
            create_ms: 1_700_000_000_000,
            target_category_code: "screen".to_string(),
            target_category_id: 11,
            // 空串 = 该类别下的全部目标（按类别广播）
            target_id: String::new(),
            channel_id: 2,
            channel_code: "websocket".to_string(),
        }
    }

    fn frame(task: &DeliveryMsg) -> serde_json::Value {
        let text = serde_json::to_string(&WsPush {
            event_code: &task.event_code,
            title: &task.title,
            content: &task.content,
            ext: task.ext.as_deref(),
            first_receive_ms: task.first_receive_ms,
            business_trigger_ms: task.business_trigger_ms,
        })
        .expect("帧序列化失败");

        serde_json::from_str(&text).expect("帧反序列化失败")
    }

    /// 帧里必须带时间锚点：大屏没有回执通道，不从帧里拿时间就只能拿本机收帧
    /// 时刻当起点，端到端延迟永远算成 0
    #[test]
    fn frame_carries_time_anchors() {
        let task = DeliveryMsg {
            first_receive_ms: Some(1_761_000_000_000),
            business_trigger_ms: Some(1_760_999_990_000),
            ..task()
        };

        let frame = frame(&task);

        assert_eq!(frame["firstReceiveMs"], 1_761_000_000_000i64);
        assert_eq!(frame["businessTriggerMs"], 1_760_999_990_000i64);
    }

    /// 两个锚点都没有时字段整段消失：大屏按「有没有」判断能不能算延迟
    #[test]
    fn absent_time_anchors_are_not_serialized() {
        let frame = frame(&task());

        assert!(frame.get("firstReceiveMs").is_none());
        assert!(frame.get("businessTriggerMs").is_none());
    }

    /// 一帧放三个渲染结果：事件编码、标题、内容
    #[test]
    fn frame_carries_render_result() {
        let frame = frame(&task());

        assert_eq!(frame["eventCode"], "position.updated");
        assert_eq!(frame["title"], "张三 进入 1F");
        // 内容是 JSON 文本，原样透传，不在推送侧二次解析
        let content: serde_json::Value =
            serde_json::from_str(frame["content"].as_str().expect("content 不是字符串"))
                .expect("content 不是合法 JSON");
        assert_eq!(content["lon"], 113.9);
    }

    /// `ext` 有就原样带下去：大屏靠它拿 alertId 与 Unity 坐标
    #[test]
    fn frame_carries_ext() {
        let task = DeliveryMsg {
            ext: Some(r#"{"alertId":"7","unityX":"1.5"}"#.to_string()),
            ..task()
        };

        let frame = frame(&task);
        let ext: serde_json::Value =
            serde_json::from_str(frame["ext"].as_str().expect("ext 不是字符串"))
                .expect("ext 不是合法 JSON");
        assert_eq!(ext["alertId"], "7");
    }

    /// 没有 `ext` 时整个字段不出现：大屏按「有没有 ext」判断，不该收到 null
    #[test]
    fn absent_ext_is_not_serialized() {
        let frame = frame(&task());
        assert!(frame.get("ext").is_none());
    }

    /// 投递元数据不下发：任务里有行 ID、目标/渠道字段，帧里一个都不能出现
    #[test]
    fn delivery_metadata_is_not_pushed() {
        let mut keys: Vec<String> = frame(&task())
            .as_object()
            .expect("帧不是对象")
            .keys()
            .cloned()
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["content", "eventCode", "title"]);
    }
}
