//! # 短信渠道发送器
//!
//! ## 为什么网关协议不写死在代码里
//!
//! 短信网关没有事实标准：报文可能是 `{"mobile","content"}`，也可能是
//! `{"phone","msg","sign"}``，响应判定可能是 `code == "0"`，也可能只看 HTTP 200。
//! 把报文和判定写进代码，每换一家网关就要发一次版；写进 `msg_channel.options`
//! 就只改一行配置（渠道写操作会自动触发消息缓存全量重建，不用重启）。
//!
//! ## options 结构
//!
//! ```json
//! {
//!   "gatewayUrl": "http://sms-gw:8080/api/send",
//!   "method": "POST",
//!   "headers": {"Authorization": "Bearer xxx"},
//!   "contentType": "application/json",
//!   "bodyTemplate": "{\"mobile\":\"{{mobile}}\",\"content\":\"{{sign}}{{content}}\"}",
//!   "successField": "code",
//!   "successValue": "0",
//!   "timeoutMs": 5000,
//!   "appId": "...",
//!   "appKey": "...",
//!   "sign": "【XX系统】",
//!   "templateId": ""
//! }
//! ```
//!
//! - `bodyTemplate` 用与消息模板同一套引擎渲染，变量见 [`render_body`]，
//!   留空时按最常见的 `{"mobile","content"}` 发；
//! - `successField` 支持 `data.status` 这类点路径，留空表示「HTTP 2xx 即成功」；
//! - `successValue` 与响应里的字段做字符串比较（数字、布尔也会转成字符串比）。
//!
//! ## 地址从哪来
//!
//! 只用 `DeliveryMsg.address`——展开阶段已经按「用户手机号优先、人员档案兜底」
//! 取好并落了 `msg_delivery_channel.address`，发送器不回查用户表，也不做二次校验
//! （格式校验在入队侧，脏号码根本不该走到这里）。
//!
//! ## 失败语义
//!
//! 短信不能丢：失败返回 [`SendOutcome::Failed`]，由 JetStream 重投
//! （`msg.delivery.sms` 队列 `persisted = true`）。重投仍失败进死信表，人工补发。

use super::{ChannelSender, SendOutcome};
use crate::cache::get_message_cache;
use crate::utils::render_template;
use async_trait::async_trait;
use msg_api::cst::MSG_CHANNEL_CODE_SMS;
use msg_api::msg::DeliveryMsg;
use reqwest::Method;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::{debug, error, warn};

/// 默认单次发送预算（毫秒）
///
/// 网关慢到这个程度说明这条短信已经错过时效，让它失败重投比干等更划算。
const DEFAULT_TIMEOUT_MS: u64 = 5_000;

/// 网关返回体在日志与失败原因里的截断长度
const BODY_BRIEF_LEN: usize = 200;

/// 短信渠道发送器
pub struct SmsSender;

#[async_trait]
impl ChannelSender for SmsSender {
    fn code(&self) -> &'static str {
        MSG_CHANNEL_CODE_SMS
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let Some(mobile) = task
            .address
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return SendOutcome::Failed(
                "短信渠道缺少收件手机号（msg_delivery_channel.address 为空）".to_string(),
            );
        };

        let options = match channel_options(&task.event_code) {
            Some(raw) => match serde_json::from_str::<SmsOptions>(&raw) {
                Ok(options) => options,
                Err(e) => {
                    error!("短信渠道 options 解析失败（请修正渠道配置）: {e}");
                    return SendOutcome::Failed(format!("短信渠道 options 解析失败: {e}"));
                }
            },
            None => {
                // 缓存里找不到渠道配置：可能是刚解绑、缓存还没重建，按默认报文发，
                // 至少不会因为一次配置变更把消息丢在发送器这一层。
                warn!(
                    "消息缓存里找不到短信渠道配置，按默认报文发送: event_code={}",
                    task.event_code
                );
                SmsOptions::default()
            }
        };

        if options.gateway_url.trim().is_empty() {
            return SendOutcome::Failed(
                "短信渠道未配置网关地址（msg_channel.options.gatewayUrl）".to_string(),
            );
        }

        let body = render_body(&options, task, mobile);
        if body.trim().is_empty() {
            return SendOutcome::Failed("短信报文渲染为空（检查 bodyTemplate）".to_string());
        }

        let method = Method::from_bytes(options.method.to_uppercase().as_bytes())
            .unwrap_or(Method::POST);
        let mut request = client()
            .request(method, options.gateway_url.trim())
            .timeout(Duration::from_millis(options.timeout_ms.max(1)))
            .header(CONTENT_TYPE, options.content_type.clone())
            .body(body);

        for (name, value) in &options.headers {
            request = request.header(name.as_str(), value.as_str());
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(e) => {
                error!(
                    "短信网关请求失败: mobile={}, event_code={}, err={e}",
                    mask(mobile),
                    task.event_code
                );
                return SendOutcome::Failed(format!("短信网关请求失败: {e}"));
            }
        };

        let status = response.status();
        let text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                return SendOutcome::Failed(format!("短信网关响应读取失败: {e}"));
            }
        };

        if !status.is_success() {
            let brief = brief(&text);
            error!(
                "短信网关返回非 2xx: mobile={}, event_code={}, status={status}, body={brief}",
                mask(mobile),
                task.event_code
            );
            return SendOutcome::Failed(format!("短信网关返回 HTTP {status}: {brief}"));
        }

        if !is_success(&options, &text) {
            let brief = brief(&text);
            warn!(
                "短信网关判定失败: mobile={}, event_code={}, body={brief}",
                mask(mobile),
                task.event_code
            );
            return SendOutcome::Failed(format!("短信网关判定失败: {brief}"));
        }

        debug!(
            "短信已提交: mobile={}, event_code={}",
            mask(mobile),
            task.event_code
        );
        SendOutcome::Delivered {
            address: format!("sms:{mobile}"),
        }
    }
}

/// # 短信渠道配置
///
/// 全部字段可缺省：缺 `bodyTemplate` 按默认报文发，缺 `successField` 按 HTTP 状态判成功。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct SmsOptions {
    /// 网关下发地址（POST JSON 或表单，由 `bodyTemplate` 与 `contentType` 决定）
    gateway_url: String,
    /// HTTP 方法，默认 POST
    method: String,
    /// 附加请求头（鉴权、租户标识这类）
    headers: HashMap<String, String>,
    /// 请求体 Content-Type，默认 application/json
    content_type: String,
    /// 请求体模板，留空用默认 `{"mobile","content"}` 报文
    body_template: String,
    /// 响应里表示结果的字段，支持 `data.status` 点路径；留空按 HTTP 2xx 判成功
    success_field: String,
    /// 成功时该字段的取值，与响应值做字符串比较，默认 `0`
    success_value: String,
    /// 单次发送超时（毫秒）
    timeout_ms: u64,
    /// 网关账号
    app_id: String,
    /// 网关密钥
    app_key: String,
    /// 短信签名，模板里用 `{{sign}}` 取
    sign: String,
    /// 模板短信的模板编号
    template_id: String,
}

impl Default for SmsOptions {
    fn default() -> Self {
        Self {
            gateway_url: String::new(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            content_type: "application/json".to_string(),
            body_template: String::new(),
            success_field: "code".to_string(),
            success_value: "0".to_string(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            app_id: String::new(),
            app_key: String::new(),
            sign: String::new(),
            template_id: String::new(),
        }
    }
}

/// 复用的 HTTP 客户端
///
/// 每条短信新建一个客户端会重新建连接池，网关批量下发时全是 TLS 握手开销。
fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(reqwest::Client::new)
}

/// 取该消息下短信渠道的 options
///
/// 渠道配置以消息缓存为准（`msg_channel.options` 随渠道写操作全量重载），
/// 不在发送路径上查库——一条报警可能同时发给几百个人，每人一次查库不值。
fn channel_options(event_code: &str) -> Option<String> {
    get_message_cache()
        .get(event_code)?
        .channels
        .iter()
        .find(|channel| channel.code == MSG_CHANNEL_CODE_SMS)?
        .options
        .clone()
}

/// # 渲染请求体
///
/// 可用变量：`mobile` / `content` / `title` / `eventCode` / `sign` / `appId` /
/// `appKey` / `templateId`，以及有值时才注入的 `ext` / `firstReceiveMs` /
/// `businessTriggerMs`（与渠道载荷模板同一套变量口径）。
///
/// 没配 `bodyTemplate` 时按最常见的网关报文来：手机号 + 「签名 + 正文」。
fn render_body(options: &SmsOptions, task: &DeliveryMsg, mobile: &str) -> String {
    if options.body_template.trim().is_empty() {
        let content = format!("{}{}", options.sign, task.content);
        return serde_json::json!({ "mobile": mobile, "content": content }).to_string();
    }

    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("mobile", mobile.to_string()),
        ("content", task.content.clone()),
        ("title", task.title.clone()),
        ("eventCode", task.event_code.clone()),
        ("sign", options.sign.clone()),
        ("appId", options.app_id.clone()),
        ("appKey", options.app_key.clone()),
        ("templateId", options.template_id.clone()),
    ]);

    if let Some(ext) = task.ext.as_deref() {
        builtins.insert("ext", ext.to_owned());
    }
    if let Some(first_receive_ms) = task.first_receive_ms {
        builtins.insert("firstReceiveMs", first_receive_ms.to_string());
    }
    if let Some(business_trigger_ms) = task.business_trigger_ms {
        builtins.insert("businessTriggerMs", business_trigger_ms.to_string());
    }

    render_template(
        &options.body_template,
        &builtins,
        &HashMap::new(),
        &HashMap::new(),
    )
}

/// # 响应是否表示成功
///
/// `successField` 留空 → HTTP 2xx 就算成功（有的网关只回 `OK` 或空体）。
/// 否则按 JSON 解析、按点路径取值，与 `successValue` 做字符串比较：
/// 网关把 `code` 写成数字 `0` 而不是字符串 `"0"` 是常态，两种都认。
fn is_success(options: &SmsOptions, body: &str) -> bool {
    if options.success_field.trim().is_empty() {
        return true;
    }

    let Ok(value) = serde_json::from_str::<Value>(body) else {
        warn!("短信网关响应不是 JSON，无法按 successField 判定");
        return false;
    };

    let found = options
        .success_field
        .split('.')
        .fold(Some(&value), |acc, key| acc.and_then(|node| node.get(key)));

    match found {
        Some(Value::String(text)) => text == &options.success_value,
        Some(Value::Number(number)) => number.to_string() == options.success_value,
        Some(Value::Bool(flag)) => flag.to_string() == options.success_value,
        _ => false,
    }
}

/// 截断响应体，避免把整个网关报文写进日志与失败原因
fn brief(text: &str) -> String {
    text.chars().take(BODY_BRIEF_LEN).collect()
}

/// 手机号打码：日志里保留前三后四，够定位一个人，又不把号码整条写进日志
fn mask(mobile: &str) -> String {
    let chars: Vec<char> = mobile.chars().collect();
    if chars.len() <= 7 {
        return "***".to_string();
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
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
            event_code: "alarm.dtw.dwell".to_string(),
            message_id: 6,
            event_source_id: 3,
            message_category_id: 1,
            business_id: None,
            business_trigger_ms: None,
            first_trigger_ms: None,
            first_receive_ms: None,
            title: "告警".to_string(),
            content: "50碱区域有人停留超时".to_string(),
            ext: None,
            create_ms: 1_795_000_000_000,
            simulated: false,
            redeliver: false,
            target_category_code: "user".to_string(),
            target_category_id: 1,
            target_id: "1279350111347933186".to_string(),
            channel_id: 4,
            channel_code: MSG_CHANNEL_CODE_SMS.to_string(),
            address: Some("13800138000".to_string()),
        }
    }

    #[test]
    fn default_body_carries_sign_and_content() {
        let options = SmsOptions {
            sign: "【测试】".to_string(),
            ..Default::default()
        };
        let body = render_body(&options, &task(), "13800138000");
        let parsed: Value = serde_json::from_str(&body).expect("默认报文必须是合法 JSON");
        assert_eq!(parsed["mobile"], "13800138000");
        assert_eq!(parsed["content"], "【测试】50碱区域有人停留超时");
    }

    #[test]
    fn body_template_renders_variables() {
        let options = SmsOptions {
            body_template: r#"{"phone":"{{mobile}}","msg":"{{sign}}{{content}}"}"#.to_string(),
            sign: "【测试】".to_string(),
            ..Default::default()
        };
        let body = render_body(&options, &task(), "13800138000");
        let parsed: Value = serde_json::from_str(&body).expect("模板渲染结果必须是合法 JSON");
        assert_eq!(parsed["phone"], "13800138000");
        assert_eq!(parsed["msg"], "【测试】50碱区域有人停留超时");
    }

    #[test]
    fn success_field_accepts_numbers_and_strings() {
        let options = SmsOptions::default();
        assert!(is_success(&options, r#"{"code":0}"#));
        assert!(is_success(&options, r#"{"code":"0"}"#));
        assert!(!is_success(&options, r#"{"code":1}"#));
        assert!(!is_success(&options, "OK"));

        let dotted = SmsOptions {
            success_field: "data.status".to_string(),
            success_value: "success".to_string(),
            ..Default::default()
        };
        assert!(is_success(&dotted, r#"{"data":{"status":"success"}}"#));
        assert!(!is_success(&dotted, r#"{"data":{"status":"fail"}}"#));
    }

    #[test]
    fn empty_success_field_only_trusts_http_status() {
        let options = SmsOptions {
            success_field: String::new(),
            ..Default::default()
        };
        assert!(is_success(&options, "OK"));
        assert!(is_success(&options, ""));
    }

    #[test]
    fn mobile_is_masked() {
        assert_eq!(mask("13800138000"), "138****8000");
        assert_eq!(mask("1234567"), "***");
    }
}
