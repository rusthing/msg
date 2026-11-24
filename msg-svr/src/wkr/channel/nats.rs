//! # NATS 渠道发送器
//!
//! 把投递任务发到一条 Core NATS subject，由消费方订阅后自己转下游协议。
//! 这里不绑定任何业务协议：主题取自渠道配置的 `subject`，报文体取自渠道的
//! **载荷模板**（`msg_channel.payload_template`）；没配模板时发一帧通用 JSON
//! （目标标识 + 消息正文 + 附加内容），协议细节全部由注册渠道的一方（dtw）
//! 在模板里写死。
//!
//! RabbitMQ 连接归 dtw 持有（契约 §1），msg-svr 不引入 RabbitMQ 依赖。
//!
//! ## options 结构
//!
//! ```json
//! {
//!   "subject": "dtw.card.callback"
//! }
//! ```
//!
//! - `subject`：Core NATS 主题，**必填**；为空判失败，不发到猜出来的主题上。
//!
//! ## 载荷模板
//!
//! `msg_channel.payload_template` 渲染整帧，可用变量：`targetId` / `eventCode` /
//! `title` / `content` / `ext` / `firstReceiveMs` / `businessTriggerMs`。
//! 模板渲染不出合法 JSON 时告警并回退通用帧，不推坏帧（下游解析失败会丢掉整帧）。
//!
//! 通用帧：
//!
//! ```json
//! {"targetId":"C001","eventCode":"alarm.dtw.enter","title":"…","content":"…","ext":{…}}
//! ```
//!
//! ## 目标从哪来
//!
//! `targetId` 取 `DeliveryMsg.target_id`，由目标展开阶段按目标类别的规则生成
//! （类别上声明了 `target_keys` 时从消息 labels 取，`allow_broadcast=false` 时
//! 空目标已被跳过）。本发送器不判断能不能广播，只按任务原样发送。
//!
//! ## 失败语义
//!
//! Core NATS 是 at-most-once：发布失败返回 [`SendOutcome::Failed`] 并打 error
//! 日志（带 subject / targetId / event_code），让持久化消息能被记录为失败；
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

/// 渠道配置：只有主题，报文结构交给载荷模板
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct NatsOptions {
    /// Core NATS 主题，必填
    subject: String,
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

        let subject = options.subject.trim();
        if subject.is_empty() {
            error!(
                "NATS 渠道未配置 subject: event_code={}, channel_type={}",
                task.event_code, task.channel_type
            );
            return SendOutcome::Failed("NATS 渠道未配置 subject".to_string());
        }

        // 配了载荷模板就按模板渲染；渲染失败/为空时回退通用帧
        if let Some(frame) = payload_template(&task.event_code, &task.channel_type)
            .and_then(|template| render_frame(&template, task))
        {
            return publish(subject, &frame, task).await;
        }

        let frame = default_frame(task);
        publish(subject, &frame, task).await
    }
}

/// 发一条 Core NATS 消息并把结果转成投递结论
async fn publish<T: Serialize + std::fmt::Debug>(
    subject: &str,
    payload: &T,
    task: &DeliveryMsg,
) -> SendOutcome {
    // Core NATS：at-most-once；投递任务已由 JetStream 队列组保证单实例消费
    match nats::publish(subject, payload, false).await {
        Ok(_) => {
            debug!(
                "NATS 渠道已发布: subject={}, targetId={}, event_code={}",
                subject, task.target_id, task.event_code
            );
            SendOutcome::Delivered {
                address: format!("nats:{subject}"),
            }
        }
        Err(e) => {
            error!(
                "NATS 渠道发布失败: subject={}, targetId={}, event_code={}, err={e}",
                subject, task.target_id, task.event_code
            );
            SendOutcome::Failed(format!("NATS 渠道发布失败: {e}"))
        }
    }
}

/// 通用帧：不绑定业务协议，模板没配时发这个
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultFrame<'a> {
    target_id: &'a str,
    event_code: &'a str,
    title: &'a str,
    content: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    ext: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_receive_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    business_trigger_ms: Option<i64>,
}

/// 组装通用帧：附加内容原样带过去，解析不出 JSON 时降级成字符串
fn default_frame(task: &DeliveryMsg) -> Value {
    let ext = task.ext.as_deref().and_then(|raw| {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        Some(serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string())))
    });
    let frame = DefaultFrame {
        target_id: task.target_id.trim(),
        event_code: &task.event_code,
        title: &task.title,
        content: &task.content,
        ext,
        first_receive_ms: task.first_receive_ms.map(|v| v as i64),
        business_trigger_ms: task.business_trigger_ms.map(|v| v as i64),
    };
    serde_json::to_value(&frame).unwrap_or(Value::Null)
}

/// 取该消息下 NATS 渠道的 options
///
/// 与短信/邮件一致：只读消息缓存，不在发送路径查库。缓存里找不到渠道配置
/// （刚解绑、缓存未重建）时按空配置处理（subject 为空 → 明确失败）；
/// options 存在但解析失败才判失败——用错 subject 静默发出去比失败更难查。
fn options_for(task: &DeliveryMsg) -> Result<NatsOptions, String> {
    let raw = get_message_cache()
        .get(&task.event_code)
        .and_then(|message| {
            message
                .channels
                .iter()
                .find(|channel| channel.channel_type == task.channel_type)
        })
        .and_then(|channel| channel.options.clone());

    match raw.as_deref().map(str::trim) {
        None | Some("") => Ok(NatsOptions::default()),
        Some(raw) => {
            serde_json::from_str(raw).map_err(|e| format!("NATS 渠道 options 解析失败: {e}"))
        }
    }
}

/// 取该渠道的载荷模板；没配或为空返回 `None`（用通用帧）
fn payload_template(event_code: &str, channel_type: &str) -> Option<String> {
    get_message_cache()
        .get(event_code)?
        .channels
        .iter()
        .find(|channel| channel.channel_type == channel_type)?
        .payload_template
        .clone()
        .filter(|template| !template.trim().is_empty())
}

/// 按载荷模板渲染整帧
///
/// 返回 `None` 表示渲染不出可用帧（空白或不是合法 JSON）：调用方回退通用帧，
/// 这里只告警，不把坏帧发出去。
fn render_frame(template: &str, task: &DeliveryMsg) -> Option<Value> {
    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("targetId", task.target_id.trim().to_string()),
        ("eventCode", task.event_code.clone()),
        ("title", task.title.clone()),
        ("content", task.content.clone()),
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
            "NATS 渠道载荷模板渲染为空，回退通用帧: event_code={}",
            task.event_code
        );
        return None;
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            warn!(
                "NATS 渠道载荷模板不是合法 JSON，回退通用帧: event_code={}, err={e}",
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
            business_trigger_ms: Some(1_700_000_100_000),
            first_trigger_ms: None,
            first_receive_ms: Some(1_700_000_200_000),
            title: "禁入报警".to_string(),
            content: "定位卡 C001 进入禁入区域".to_string(),
            ext: Some(r#"{"regionId":12}"#.to_string()),
            create_ms: 1_700_000_000_000,
            simulated: false,
            redeliver: false,
            target_category_code: "locator".to_string(),
            target_category_id: 11,
            target_id: "C001".to_string(),
            channel_id: 3,
            channel_type: "nats".to_string(),
            address: None,
        }
    }

    /// 渠道类型与 cst、msg_channel.channel_type、投递队列末段一致
    #[test]
    fn channel_is_nats() {
        assert_eq!(NatsSender.channel(), ChannelCode::Nats);
    }

    /// options 只有 subject；缺省时为空，由 send 明确判失败
    #[test]
    fn subject_is_required() {
        assert!(NatsOptions::default().subject.trim().is_empty());
        let parsed: NatsOptions =
            serde_json::from_str(r#"{"subject":"dtw.card.callback"}"#).expect("options 解析失败");
        assert_eq!(parsed.subject, "dtw.card.callback");
    }

    /// 通用帧字段名 camelCase，附加内容原样带上
    #[test]
    fn default_frame_is_generic() {
        let value = default_frame(&task());
        assert_eq!(value["targetId"], "C001");
        assert_eq!(value["eventCode"], "alarm.dtw.enter");
        assert_eq!(value["title"], "禁入报警");
        assert_eq!(value["content"], "定位卡 C001 进入禁入区域");
        assert_eq!(value["ext"]["regionId"], 12);
        assert_eq!(value["firstReceiveMs"], 1_700_000_200_000i64);
        assert_eq!(value["businessTriggerMs"], 1_700_000_100_000i64);
    }

    /// 附加内容不是 JSON 时降级成字符串，不丢
    #[test]
    fn default_frame_keeps_non_json_ext() {
        let task = DeliveryMsg {
            ext: Some("plain".to_string()),
            ..task()
        };
        assert_eq!(default_frame(&task)["ext"], "plain");
    }

    /// 载荷模板按注册方写的协议渲染：targetId → deviceId
    #[test]
    fn payload_template_renders_protocol_frame() {
        let template = r#"{"deviceId":"{{targetId}}","code":0,"message":"{{title}} {{content}}"}"#;
        let frame = render_frame(template, &task()).expect("模板渲染失败");
        assert_eq!(frame["deviceId"], "C001");
        assert_eq!(frame["code"], 0);
        assert_eq!(frame["message"], "禁入报警 定位卡 C001 进入禁入区域");
    }

    /// 模板不是合法 JSON 时回退通用帧，不推坏帧
    #[test]
    fn broken_template_falls_back_to_none() {
        assert!(render_frame(r#"{"deviceId":"{{targetId}}""#, &task()).is_none());
        assert!(render_frame("   ", &task()).is_none());
    }
}
