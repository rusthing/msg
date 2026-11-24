//! # NATS 渠道发送器
//!
//! 把投递任务发到一条 Core NATS subject，由消费方订阅后自己转下游协议。
//! RabbitMQ 连接归 dtw 持有（契约 §1），msg-svr 不引入 RabbitMQ 依赖：默认
//! 只发 **`dtw.card.callback`**，dtw 订阅后写 `queue_card_callback`。
//!
//! 渠道类型从历史上叫过 `locator`，现在按实际行为叫 `nats`：换一条 subject
//! 就能对接别的消费方，不用发版。
//!
//! ## options 结构
//!
//! ```json
//! {
//!   "subject": "dtw.card.callback",
//!   "code": 0,
//!   "messageTemplate": "{{title}} {{content}}"
//! }
//! ```
//!
//! - `subject`：Core NATS 主题，留空用默认 `dtw.card.callback`；
//! - `code`：默认报文里的 `code` 字段。对定位卡是震动方式，`0` 表示红蓝双灯
//!   同时震动（协议 §2.4）；
//! - `messageTemplate`：默认报文里的 `message` 文案，留空用 `title + 空格 + content`。
//!   可用变量：`eventCode` / `title` / `content` / `deviceId` / `code`。
//!
//! 要换整个报文结构时配渠道的**载荷模板**（`msg_channel.payload_template`），
//! 可用变量：`deviceId` / `code` / `message` / `eventCode` / `title` / `content` /
//! `ext` / `firstReceiveMs` / `businessTriggerMs`。模板渲染不出合法 JSON 时告警
//! 并回退默认报文，不推坏帧（下游解析失败会丢掉整帧，比旧结构更糟）。
//!
//! 默认报文（定位卡协议）：
//!
//! ```json
//! {"deviceId":"C001","code":0,"message":"…"}
//! ```
//!
//! ## 目标从哪来
//!
//! `deviceId` 取 `DeliveryMsg.target_id`。默认报文**只允许单卡**：广播已禁用，
//! 缺少具体目标直接判失败，不再按协议换成广播码 `FFFFFFFF`。配了载荷模板且
//! 模板不使用 `deviceId` 时允许空目标——消费方不需要的标识不该把人卡住。
//!
//! 契约 §4 提到 gather 按 `labels.cardCodes` 精确投卡：目标展开阶段
//! （`target_expand`）已经把发布方的 labels 展开成逐卡目标，本发送器只保证
//! 每个任务一个卡号，原样落到 `deviceId`；这里不再做任何拆分或广播。
//!
//! ## 失败语义
//!
//! Core NATS 是 at-most-once：发布失败返回 [`SendOutcome::Failed`] 并打 error
//! 日志（带 subject / deviceId / event_code），让持久化消息能被记录为失败；
//! 非持久化消息按执行器语义丢弃。这里不重试、不落库。

use super::{ChannelSender, SendOutcome};
use crate::cache::get_message_cache;
use crate::utils::render_template;
use async_trait::async_trait;
use msg_api::cst::ChannelCode;
use msg_api::msg::DeliveryMsg;
use robotech::mq::nats;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, warn};

/// 默认 subject（dtw 订阅后写 `queue_card_callback`）
const DEFAULT_SUBJECT: &str = "dtw.card.callback";

/// 渠道配置：subject、震动码与文案模板
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct NatsOptions {
    /// Core NATS 主题
    subject: String,
    /// 默认报文里的 `code`（定位卡：0 = 红蓝双灯同时震动）
    code: u8,
    /// 默认报文里的 `message` 文案模板，留空用 title + content
    message_template: String,
}

impl Default for NatsOptions {
    fn default() -> Self {
        Self {
            subject: DEFAULT_SUBJECT.to_string(),
            code: 0,
            message_template: String::new(),
        }
    }
}

/// NATS 渠道发送器
pub struct NatsSender;

#[async_trait]
impl ChannelSender for NatsSender {
    fn channel(&self) -> ChannelCode {
        ChannelCode::Nats
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let options = match options_for(task) {
            Ok(options) => options,
            Err(reason) => {
                error!(
                    "NATS 渠道 options 解析失败（请修正渠道配置）: event_code={}, err={reason}",
                    task.event_code
                );
                return SendOutcome::Failed(reason);
            }
        };
        let subject = subject_of(&options);
        let device_id = task.target_id.trim();
        let message = compose_message(&options, task);

        // 配了载荷模板就按模板渲染；渲染失败/为空时回退默认报文
        if let Some(frame) = payload_template(&task.event_code, &task.channel_code)
            .and_then(|template| render_frame(&template, task, device_id, &options, &message))
        {
            return publish(subject, &frame, device_id, task).await;
        }

        // 默认报文需要具体卡号：广播已禁用，空目标直接判失败
        let Some(device_id) = (!device_id.is_empty()).then_some(device_id) else {
            error!(
                "NATS 渠道缺少目标标识（广播已禁用）: event_code={}, subject={subject}",
                task.event_code
            );
            return SendOutcome::Failed("NATS 渠道缺少目标标识（广播已禁用）".to_string());
        };
        let payload = CardCallback {
            device_id,
            code: options.code,
            message: &message,
        };
        publish(subject, &payload, device_id, task).await
    }
}

/// 发一条 Core NATS 消息并把结果转成投递结论
async fn publish<T: Serialize + std::fmt::Debug>(
    subject: &str,
    payload: &T,
    device_id: &str,
    task: &DeliveryMsg,
) -> SendOutcome {
    // Core NATS：at-most-once；投递任务已由 JetStream 队列组保证单实例消费
    match nats::publish(subject, payload, false).await {
        Ok(_) => {
            debug!(
                "NATS 渠道已发布: subject={}, deviceId={}, event_code={}",
                subject, device_id, task.event_code
            );
            SendOutcome::Delivered {
                address: format!("nats:{subject}"),
            }
        }
        Err(e) => {
            error!(
                "NATS 渠道发布失败: subject={}, deviceId={}, event_code={}, err={e}",
                subject, device_id, task.event_code
            );
            SendOutcome::Failed(format!("NATS 渠道发布失败: {e}"))
        }
    }
}

/// 默认报文：`#[serde(rename_all = "camelCase")]` 把 `device_id` 落成协议要求的 `deviceId`
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardCallback<'a> {
    device_id: &'a str,
    code: u8,
    message: &'a str,
}

/// 取该消息下 NATS 渠道的 options
///
/// 与短信/邮件一致：只读消息缓存，不在发送路径查库。缓存里找不到渠道配置
/// （刚解绑、缓存未重建）时按默认配置发，不因为一次配置变更把消息丢在发送器
/// 这一层；options 存在但解析失败才判失败——用错 subject 静默发出去比失败更难查。
fn options_for(task: &DeliveryMsg) -> Result<NatsOptions, String> {
    let raw = get_message_cache()
        .get(&task.event_code)
        .and_then(|message| {
            message
                .channels
                .iter()
                .find(|channel| channel.code == task.channel_code)
        })
        .and_then(|channel| channel.options.clone());

    match raw.as_deref().map(str::trim) {
        None | Some("") => Ok(NatsOptions::default()),
        Some(raw) => {
            serde_json::from_str(raw).map_err(|e| format!("NATS 渠道 options 解析失败: {e}"))
        }
    }
}

/// subject 留空即默认值；两边空白一并去掉，避免发出带空格的主题
fn subject_of(options: &NatsOptions) -> &str {
    let subject = options.subject.trim();
    if subject.is_empty() {
        DEFAULT_SUBJECT
    } else {
        subject
    }
}

/// 文案：配了模板按模板渲染，留空或渲染为空时用 `title + 空格 + content`
fn compose_message(options: &NatsOptions, task: &DeliveryMsg) -> String {
    if !options.message_template.trim().is_empty() {
        let builtins: HashMap<&'static str, String> = HashMap::from([
            ("eventCode", task.event_code.clone()),
            ("title", task.title.clone()),
            ("content", task.content.clone()),
            ("deviceId", task.target_id.trim().to_string()),
            ("code", options.code.to_string()),
        ]);
        let rendered = render_template(
            &options.message_template,
            &builtins,
            &HashMap::new(),
            &HashMap::new(),
        );
        let rendered = rendered.trim();
        if !rendered.is_empty() {
            return rendered.to_string();
        }
        warn!(
            "NATS 渠道 messageTemplate 渲染为空，改用 title + content: event_code={}",
            task.event_code
        );
    }
    [task.title.trim(), task.content.trim()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// 取该渠道的载荷模板；没配或为空返回 `None`（用默认报文）
fn payload_template(event_code: &str, channel_code: &str) -> Option<String> {
    get_message_cache()
        .get(event_code)?
        .channels
        .iter()
        .find(|channel| channel.code == channel_code)?
        .payload_template
        .clone()
        .filter(|template| !template.trim().is_empty())
}

/// 按载荷模板渲染整帧
///
/// 返回 `None` 表示渲染不出可用帧（空白或不是合法 JSON）：调用方回退默认报文，
/// 这里只告警，不把坏帧发出去。
fn render_frame(
    template: &str,
    task: &DeliveryMsg,
    device_id: &str,
    options: &NatsOptions,
    message: &str,
) -> Option<Value> {
    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("eventCode", task.event_code.clone()),
        ("title", task.title.clone()),
        ("content", task.content.clone()),
        ("deviceId", device_id.to_string()),
        ("code", options.code.to_string()),
        ("message", message.to_string()),
    ]);
    // 缺的变量不注入：模板里 `{{ext:json}}` 渲染成 null，与「没有附加内容」一致
    if let Some(ext) = task.ext.as_deref() {
        builtins.insert("ext", ext.to_owned());
    }
    if let Some(ms) = task.first_receive_ms {
        builtins.insert("firstReceiveMs", ms.to_string());
    }
    if let Some(ms) = task.business_trigger_ms {
        builtins.insert("businessTriggerMs", ms.to_string());
    }

    let text = render_template(template, &builtins, &HashMap::new(), &HashMap::new());
    if text.trim().is_empty() {
        warn!(
            "NATS 渠道载荷模板渲染为空，回退默认报文: event_code={}",
            task.event_code
        );
        return None;
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            warn!(
                "NATS 渠道载荷模板不是合法 JSON，回退默认报文: event_code={}, err={e}",
                task.event_code
            );
            None
        }
    }
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
            channel_id: 3,
            channel_code: "nats".to_string(),
            channel_type: "nats".to_string(),
            address: None,
        }
    }

    /// 渠道类型与 cst、msg_channel.code、投递队列末段一致
    #[test]
    fn channel_is_nats() {
        assert_eq!(NatsSender.channel(), ChannelCode::Nats);
    }

    /// 默认配置：subject 与震动码取协议默认值
    #[test]
    fn default_options_use_card_protocol_defaults() {
        let options = NatsOptions::default();
        assert_eq!(subject_of(&options), DEFAULT_SUBJECT);
        assert_eq!(options.code, 0);
        assert!(options.message_template.is_empty());
    }

    /// 缺字段（只配 subject）时其余字段用默认值
    #[test]
    fn partial_options_fall_back_to_defaults() {
        let options: NatsOptions =
            serde_json::from_str(r#"{"subject":"dtw.other"}"#).expect("options 解析失败");
        assert_eq!(subject_of(&options), "dtw.other");
        assert_eq!(options.code, 0);
        assert!(options.message_template.is_empty());
    }

    /// subject 留空/空白时用默认主题
    #[test]
    fn blank_subject_falls_back_to_default() {
        let options = NatsOptions {
            subject: "  ".to_string(),
            ..NatsOptions::default()
        };
        assert_eq!(subject_of(&options), DEFAULT_SUBJECT);
    }

    /// 报文字段名与大小写严格对齐懋特协议：deviceId / code / message
    #[test]
    fn payload_field_names_match_protocol() {
        let task = DeliveryMsg {
            target_id: "C001".to_string(),
            ..task()
        };
        let options = NatsOptions::default();
        let message = compose_message(&options, &task);
        let payload = CardCallback {
            device_id: task.target_id.trim(),
            code: options.code,
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

    /// 文案拼接：空段跳过，不留下多余空格
    #[test]
    fn compose_message_skips_blank_parts() {
        let task = DeliveryMsg {
            title: "  ".to_string(),
            content: " 仅内容 ".to_string(),
            ..task()
        };
        assert_eq!(compose_message(&NatsOptions::default(), &task), "仅内容");
    }

    /// messageTemplate 覆盖默认拼接，可引用 deviceId / code
    #[test]
    fn message_template_overrides_default() {
        let task = DeliveryMsg {
            target_id: "C001".to_string(),
            ..task()
        };
        let options = NatsOptions {
            message_template: "{{code}}#{{deviceId}} {{title}}".to_string(),
            ..NatsOptions::default()
        };
        assert_eq!(compose_message(&options, &task), "0#C001 禁入报警");
    }
}
