//! # 定位卡渠道发送器
//!
//! ## 为什么是「发 NATS」而不是「写 RabbitMQ」
//!
//! 懋特下行协议要求第三方平台往 RabbitMQ 队列 `queue_card_callback` 发
//! `{"deviceId","code":0,"message"}` 让卡片红蓝双灯同时震动。RabbitMQ 连接归
//! dtw 持有（契约 §1），msg-svr 不引入 RabbitMQ 依赖：这里只把投递任务转成一条
//! Core NATS 消息发到 **`dtw.card.callback`**，由 dtw 订阅后写那个队列。
//!
//! ## 字段从哪来
//!
//! - `deviceId`：`DeliveryMsg.target_id`。空串 = 该类别下全部目标，按协议换成
//!   广播码 `FFFFFFFF`；非空就是卡号原样下发；
//! - `code`：固定 `0`（卡片红蓝双灯同时震动，协议 §2.4）；
//! - `message`：`title` 与 `content` 拼成的通知文案。
//!
//! 契约 §4 提到 gather 按 `labels.cardCodes` 精确投卡：投递任务（`DeliveryMsg`）
//! 不带发布方的 labels，精确投卡由 dtw 订阅 `dtw.card.callback` 后处理；msg 侧
//! 只保证 `target_id` 语义（空串广播）原样落到 `deviceId`。
//!
//! ## 失败语义
//!
//! Core NATS 是 at-most-once：发布失败返回 [`SendOutcome::Failed`] 并打 error
//! 日志（带 subject / deviceId / event_code），让持久化消息能被记录为失败；
//! 非持久化消息按执行器语义丢弃。这里不重试、不落库。

use super::{ChannelSender, SendOutcome};
use async_trait::async_trait;
use msg_api::cst::MSG_CHANNEL_CODE_LOCATOR_CARD;
use msg_api::msg::DeliveryMsg;
use robotech::mq::nats;
use serde::Serialize;
use tracing::{debug, error};

/// 定位卡下行广播 subject（dtw 订阅后写 `queue_card_callback`）
const CARD_CALLBACK_SUBJECT: &str = "dtw.card.callback";
/// 协议约定的广播卡号：`target_id` 为空串时用它，表示全部卡片
const CARD_BROADCAST_ID: &str = "FFFFFFFF";

/// 定位卡渠道发送器
pub struct LocatorCardSender;

#[async_trait]
impl ChannelSender for LocatorCardSender {
    fn code(&self) -> &'static str {
        MSG_CHANNEL_CODE_LOCATOR_CARD
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let message = compose_message(task);
        let device_id = device_id(task);
        let payload = CardCallback {
            device_id,
            code: 0,
            message: &message,
        };

        // Core NATS：at-most-once；投递任务已由 JetStream 队列组保证单实例消费
        match nats::publish(CARD_CALLBACK_SUBJECT, &payload, false).await {
            Ok(_) => {
                debug!(
                    "定位卡下行已发布: subject={}, deviceId={}, event_code={}",
                    CARD_CALLBACK_SUBJECT, device_id, task.event_code
                );
                SendOutcome::Delivered {
                    address: format!("nats:{CARD_CALLBACK_SUBJECT}"),
                }
            }
            Err(e) => {
                error!(
                    "定位卡下行发布失败: subject={}, deviceId={}, event_code={}, err={e}",
                    CARD_CALLBACK_SUBJECT, device_id, task.event_code
                );
                SendOutcome::Failed(format!("定位卡下行发布失败: {e}"))
            }
        }
    }
}

/// 发给卡片的下行报文：`#[serde(rename_all = "camelCase")]` 把 `device_id`
/// 落成协议要求的 `deviceId`。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardCallback<'a> {
    device_id: &'a str,
    code: u8,
    message: &'a str,
}

/// `target_id` → 协议 `deviceId`：空串换广播码，非空原样当卡号。
fn device_id(task: &DeliveryMsg) -> &str {
    if task.target_id.is_empty() {
        CARD_BROADCAST_ID
    } else {
        task.target_id.as_str()
    }
}

/// `title` + `content` 拼成一条给卡片显示的通知文案，空段跳过、用空格分隔。
fn compose_message(task: &DeliveryMsg) -> String {
    [task.title.trim(), task.content.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
            event_code: "alarm.dtw.enter".to_string(),
            message_id: 7,
            event_source_id: 3,
            message_category_id: 5,
            business_id: None,
            business_trigger_ms: None,
            first_trigger_ms: None,
            first_receive_ms: None,
            title: "禁入报警".to_string(),
            content: "定位卡 C001 进入禁入区域".to_string(),
            ext: None,
            create_ms: 1_700_000_000_000,
            simulated: false,
            redeliver: false,
            target_category_code: "locator".to_string(),
            target_category_id: 11,
            target_id: String::new(),
            channel_id: 2,
            channel_code: "locator".to_string(),
        }
    }

    /// 渠道编码与 cst、msg_channel.code、投递队列末段一致
    #[test]
    fn code_is_locator() {
        assert_eq!(LocatorCardSender.code(), "locator");
    }

    /// 空 target_id 落成广播码
    #[test]
    fn empty_target_is_broadcast() {
        assert_eq!(device_id(&task()), CARD_BROADCAST_ID);
    }

    /// 非空 target_id 原样作为卡号
    #[test]
    fn non_empty_target_is_device_id() {
        let task = DeliveryMsg {
            target_id: "C001".to_string(),
            ..task()
        };
        assert_eq!(device_id(&task), "C001");
    }

    /// 报文字段名与大小写严格对齐懋特协议：deviceId / code / message
    #[test]
    fn payload_field_names_match_protocol() {
        let task = DeliveryMsg {
            target_id: "C001".to_string(),
            ..task()
        };
        let message = compose_message(&task);
        let payload = CardCallback {
            device_id: device_id(&task),
            code: 0,
            message: &message,
        };
        let value = serde_json::to_value(&payload).expect("报文序列化失败");

        assert_eq!(value["deviceId"], "C001");
        assert_eq!(value["code"], 0);
        assert_eq!(value["message"], "禁入报警 定位卡 C001 进入禁入区域");
        // 没有多余的蛇形字段，也没有多出来的字段
        assert!(value.get("device_id").is_none());
        assert_eq!(value.as_object().expect("报文不是对象").len(), 3);
    }

    /// 空 target_id 的报文里 deviceId 必须是 FFFFFFFF
    #[test]
    fn broadcast_payload_uses_ffffffff() {
        let task = task();
        let message = compose_message(&task);
        let payload = CardCallback {
            device_id: device_id(&task),
            code: 0,
            message: &message,
        };
        let value = serde_json::to_value(&payload).expect("报文序列化失败");
        assert_eq!(value["deviceId"], CARD_BROADCAST_ID);
    }

    /// 文案拼接：空段跳过，不留下多余空格
    #[test]
    fn compose_message_skips_blank_parts() {
        let task = DeliveryMsg {
            title: "  ".to_string(),
            content: " 仅内容 ".to_string(),
            ..task()
        };
        assert_eq!(compose_message(&task), "仅内容");
    }
}
