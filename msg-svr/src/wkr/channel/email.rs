//! # 邮件渠道发送器
//!
//! ## 为什么不引第三方 SMTP 库
//!
//! `lettre` 这类库默认带 TLS，而本项目的 musl 静态交叉编译对 TLS 栈极不友好
//! （reqwest 那次已经付过一次学费：aws-lc-rs 要 cmake/nasm，openssl 要 vendored）。
//! 内网邮件网关通常是 25 端口的明文 SMTP，加认证也就是 `AUTH PLAIN` / `AUTH LOGIN`。
//! 所以这里直接按 RFC 5321 写一个最小会话：`EHLO` → 认证 → `MAIL FROM` → `RCPT TO`
//! → `DATA` → `QUIT`，零新增依赖。
//!
//! 代价是**不支持 STARTTLS/465 隐式 TLS**：`options.requireTls = true` 会直接失败并
//! 说明原因，而不是悄悄用明文把邮件发出去。
//!
//! ## options 结构
//!
//! ```json
//! {
//!   "smtpHost": "mail.example.com",
//!   "smtpPort": 25,
//!   "username": "notice@example.com",
//!   "password": "***",
//!   "fromAddress": "notice@example.com",
//!   "fromName": "消息中心",
//!   "contentType": "text/html",
//!   "subjectTemplate": "【告警】{{title}}",
//!   "bodyTemplate": "<h3>{{title}}</h3><p>{{content}}</p>",
//!   "timeoutMs": 10000,
//!   "requireTls": false
//! }
//! ```
//!
//! - 全部字段可缺省：缺 `subjectTemplate` 用投递任务的 `title`，缺 `bodyTemplate`
//!   用 `content`（纯文本按 `text/plain` 发）；
//! - `username` 为空表示不需要认证（内网中继常见）；
//! - 正文与主题都按 UTF-8 base64 编码，避免中文被网关截断成 `?`。
//!
//! ## 地址从哪来
//!
//! 只用 `DeliveryMsg.address`，落库为 `email:<地址>`。格式不合法（没有 `@` 或含空白）
//! 直接判失败，不浪费时间去连 SMTP——脏邮箱在入队侧就该被拦下。
//!
//! ## 失败语义
//!
//! 邮件同样不能丢：任何非预期 SMTP 码都返回 [`SendOutcome::Failed`]，由 JetStream 重投
//! （`msg.delivery.email` 队列 `persisted = true`），重投仍失败进死信表。

use super::{ChannelSender, SendOutcome};
use crate::cache::get_message_cache;
use crate::utils::render_template;
use async_trait::async_trait;
use chrono::Utc;
use msg_api::cst::ChannelCode;
use msg_api::msg::DeliveryMsg;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::time::timeout;
use tracing::{debug, error, warn};

/// 默认单次会话预算（毫秒）
const DEFAULT_TIMEOUT_MS: u64 = 10_000;

/// SMTP 默认端口
const DEFAULT_SMTP_PORT: u16 = 25;

/// 邮件头一段 encoded-word 的 base64 长度上限
///
/// 48 是 4 的倍数（每段恰好 36 字节），不会把 UTF-8 的多字节字符切断；
/// 叠上 `=?UTF-8?B??=` 的 12 个字符后仍在一行 78 字符的常规限制内。
const ENCODED_WORD_CHUNK: usize = 48;

/// 正文 base64 每行长度
const BODY_LINE_LEN: usize = 76;

/// 邮件渠道发送器
pub struct EmailSender;

#[async_trait]
impl ChannelSender for EmailSender {
    fn channel(&self) -> ChannelCode {
        ChannelCode::Email
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let Some(to) = task
            .address
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return SendOutcome::Failed(
                "邮件渠道缺少收件地址（msg_delivery_channel.address 为空）".to_string(),
            );
        };

        if !is_email(to) {
            return SendOutcome::Failed(format!("收件邮箱格式非法: {}", mask(to)));
        }

        let options = match channel_options(&task.event_code, &task.channel_type) {
            Some(raw) => match serde_json::from_str::<EmailOptions>(&raw) {
                Ok(options) => options,
                Err(e) => {
                    error!("邮件渠道 options 解析失败（请修正渠道配置）: {e}");
                    return SendOutcome::Failed(format!("邮件渠道 options 解析失败: {e}"));
                }
            },
            None => {
                // 缓存里找不到配置（刚解绑 / 缓存未重建）：按默认 topic 发，
                // 至少不因为一次配置变更把消息丢在发送器这一层。
                warn!(
                    "消息缓存里找不到邮件渠道配置，按默认报文发送: event_code={}",
                    task.event_code
                );
                EmailOptions::default()
            }
        };

        if options.smtp_host.trim().is_empty() {
            return SendOutcome::Failed(
                "邮件渠道未配置 SMTP 主机（msg_channel.options.smtpHost）".to_string(),
            );
        }

        if options.require_tls {
            return SendOutcome::Failed(
                "邮件渠道要求 TLS，但当前构建未启用 TLS 特性（内网明文 SMTP 请把 requireTls 置 false）"
                    .to_string(),
            );
        }

        // 465 是隐式 TLS 端口：不加密地连上去，网关要么直接断开、要么等 TLS 握手到超时。
        // 这种配置错误必须在连接前就说清楚，别让它表现为「邮件发送超时」。
        if options.smtp_port == 465 {
            return SendOutcome::Failed(
                "邮件渠道端口 465 需要隐式 TLS，当前构建不支持；请改用明文端口（常见 25 或 587，且不启用 STARTTLS）"
                    .to_string(),
            );
        }

        let message = build_message(&options, task, to);
        let budget = Duration::from_millis(options.timeout_ms.max(1_000));

        match timeout(budget, deliver(&options, to, &message)).await {
            Ok(Ok(_)) => {
                debug!(
                    "邮件已提交: to={}, event_code={}",
                    mask(to),
                    task.event_code
                );
                SendOutcome::Delivered {
                    address: format!("email:{to}"),
                }
            }
            Ok(Err(reason)) => {
                error!(
                    "邮件发送失败: to={}, event_code={}, reason={reason}",
                    mask(to),
                    task.event_code
                );
                SendOutcome::Failed(format!("邮件发送失败: {reason}"))
            }
            Err(_) => {
                error!(
                    "邮件发送超时: to={}, event_code={}",
                    mask(to),
                    task.event_code
                );
                SendOutcome::Failed(format!("邮件发送超时（{}ms）", options.timeout_ms))
            }
        }
    }
}

/// # 邮件渠道配置
///
/// 全部字段可缺省：缺 `subjectTemplate`/`bodyTemplate` 时直接用投递任务的标题与正文。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct EmailOptions {
    /// SMTP 服务器地址
    smtp_host: String,
    /// SMTP 端口，默认 25
    smtp_port: u16,
    /// 认证用户名，留空表示不需要认证
    username: String,
    /// 认证密码
    password: String,
    /// 发件地址（信封 `MAIL FROM` 与头 `From` 都用它）
    from_address: String,
    /// 发件人显示名
    from_name: String,
    /// 正文 Content-Type，默认 text/plain
    content_type: String,
    /// 主题模板，留空用投递任务的 `title`
    subject_template: String,
    /// 正文模板，留空用投递任务的 `content`
    body_template: String,
    /// 单次会话超时（毫秒）
    timeout_ms: u64,
    /// 是否要求 TLS——当前构建不支持，置 true 会直接失败
    require_tls: bool,
}

impl Default for EmailOptions {
    fn default() -> Self {
        Self {
            smtp_host: String::new(),
            smtp_port: DEFAULT_SMTP_PORT,
            username: String::new(),
            password: String::new(),
            from_address: String::new(),
            from_name: String::new(),
            content_type: "text/plain".to_string(),
            subject_template: String::new(),
            body_template: String::new(),
            timeout_ms: DEFAULT_TIMEOUT_MS,
            require_tls: false,
        }
    }
}

/// 取该消息下邮件渠道的 options（与短信一致：只走消息缓存，不在发送路径查库）
fn channel_options(event_code: &str, channel_type: &str) -> Option<String> {
    get_message_cache()
        .get(event_code)?
        .channels
        .iter()
        .find(|channel| channel.channel_type == channel_type)?
        .options
        .clone()
}

/// # 组装一封完整的 MIME 邮件
///
/// 主题、正文、发件人名都用 UTF-8 base64 编码（`=?UTF-8?B?…?=` / `Content-Transfer-Encoding: base64`），
/// 这是国内邮件网关兼容性最好的一种写法——直接塞 UTF-8 原文，很多老网关会显示乱码。
fn build_message(options: &EmailOptions, task: &DeliveryMsg, to: &str) -> String {
    let subject = render_field(&options.subject_template, options, task, &task.title);
    let body = render_field(&options.body_template, options, task, &task.content);
    let from_address = if options.from_address.trim().is_empty() {
        "noreply@localhost"
    } else {
        options.from_address.trim()
    };

    let mut headers = String::new();
    headers.push_str(&format!(
        "From: {}\r\n",
        format_mailbox(&options.from_name, from_address)
    ));
    headers.push_str(&format!("To: <{to}>\r\n"));
    headers.push_str(&format!(
        "Subject: {}\r\n",
        encoded_word(if subject.trim().is_empty() {
            task.event_code.as_str()
        } else {
            subject.as_str()
        })
    ));
    headers.push_str(&format!("Date: {}\r\n", Utc::now().to_rfc2822()));
    headers.push_str(&format!(
        "Message-ID: <{}.{}@{}>\r\n",
        Utc::now().timestamp_millis(),
        unique_suffix(),
        domain_of(from_address)
    ));
    headers.push_str("MIME-Version: 1.0\r\n");
    headers.push_str(&format!(
        "Content-Type: {}; charset=UTF-8\r\n",
        options.content_type.trim()
    ));
    headers.push_str("Content-Transfer-Encoding: base64\r\n");
    headers.push_str("\r\n");

    let encoded = wrap_base64(&base64(body.as_bytes()), BODY_LINE_LEN);
    format!("{headers}{encoded}\r\n")
}

/// 渲染主题/正文：模板非空按模板（`{{title}}`/`{{content}}`/`{{eventCode}}`/`{{sign}}` 等），
/// 空白则退回任务自带字段。
fn render_field(
    template: &str,
    options: &EmailOptions,
    task: &DeliveryMsg,
    fallback: &str,
) -> String {
    if template.trim().is_empty() {
        return fallback.to_string();
    }

    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("content", task.content.clone()),
        ("title", task.title.clone()),
        ("eventCode", task.event_code.clone()),
        ("fromName", options.from_name.clone()),
        ("fromAddress", options.from_address.clone()),
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

    render_template(template, &builtins, &HashMap::new(), &HashMap::new())
}

/// `From:` 的邮箱形式：有显示名就用 encoded-word，没有就只写地址
fn format_mailbox(name: &str, address: &str) -> String {
    if name.trim().is_empty() {
        format!("<{address}>")
    } else {
        format!("{} <{address}>", encoded_word(name.trim()))
    }
}

/// # RFC 2047 encoded-word
///
/// 按 48 个 base64 字符切段，段间用 `\r\n ` 折行；单段时就是 `=?UTF-8?B?xxx?=`。
fn encoded_word(text: &str) -> String {
    let encoded = base64(text.as_bytes());
    let chunks: Vec<String> = encoded
        .as_bytes()
        .chunks(ENCODED_WORD_CHUNK)
        .map(|chunk| format!("=?UTF-8?B?{}?=", String::from_utf8_lossy(chunk)))
        .collect();
    chunks.join("\r\n ")
}

/// 正文按固定宽度折行，符合 SMTP 的 998 字节上限（这里取更保守的 76）
fn wrap_base64(encoded: &str, width: usize) -> String {
    encoded
        .as_bytes()
        .chunks(width)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

/// 从发件地址取域名，用于 `Message-ID`；取不到就用 `localhost`
fn domain_of(address: &str) -> &str {
    address
        .rsplit_once('@')
        .map(|(_, domain)| domain)
        .unwrap_or("localhost")
}

/// `Message-ID` 的去重后缀（进程内单调递增 + 时间戳，够用且不引随机数依赖）
fn unique_suffix() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// 邮箱格式的基本校验：有且仅有一个 `@`、两侧非空、不含空白与控制字符
///
/// 不做完整 RFC 5322 校验——真正的合法性只能靠 SMTP 服务器 `RCPT TO` 的应答，
/// 这里只是把明显脏的数据挡在建立连接之前。
fn is_email(address: &str) -> bool {
    if address.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    match address.split_once('@') {
        Some((local, domain)) => !local.is_empty() && !domain.is_empty() && !domain.contains('@'),
        None => false,
    }
}

/// # 与 SMTP 服务器完成一次投递
///
/// 只做明文会话：`220` → `EHLO` → 可选认证 → `MAIL FROM` → `RCPT TO` → `DATA` → `QUIT`。
/// 每个非预期应答都带上 RFC 要求的完整文本，方便运维判断是「账号错」还是「被反垃圾拦了」。
async fn deliver(options: &EmailOptions, to: &str, message: &str) -> Result<String, String> {
    let host = options.smtp_host.trim();
    let port = if options.smtp_port == 0 {
        DEFAULT_SMTP_PORT
    } else {
        options.smtp_port
    };

    let stream = TcpStream::connect(format!("{host}:{port}"))
        .await
        .map_err(|e| format!("连接 {host}:{port} 失败: {e}"))?;
    let _ = stream.set_nodelay(true);
    let (read_half, write_half) = stream.into_split();
    let mut session = Session {
        reader: BufReader::new(read_half),
        writer: write_half,
    };

    expect(&mut session, 220, "服务器问候").await?;

    let helo_name = domain_of(if options.from_address.trim().is_empty() {
        "noreply@localhost"
    } else {
        options.from_address.trim()
    });
    let capabilities = session.command(&format!("EHLO {helo_name}"), 250).await?;

    if !options.username.trim().is_empty() {
        authenticate(&mut session, options, &capabilities).await?;
    }

    let from = if options.from_address.trim().is_empty() {
        options.username.trim()
    } else {
        options.from_address.trim()
    };
    if from.is_empty() {
        return Err("未配置发件地址（options.fromAddress）".to_string());
    }
    session.command(&format!("MAIL FROM:<{from}>"), 250).await?;
    // RCPT 的合法应答是 250 或 251（用户不在本地域但会转发）
    session
        .command_any(&format!("RCPT TO:<{to}>"), &[250, 251])
        .await?;
    let data_reply = session.command("DATA", 354).await?;
    let _ = data_reply;

    session
        .writer
        .write_all(message.as_bytes())
        .await
        .map_err(|e| format!("写入邮件正文失败: {e}"))?;
    session
        .writer
        .write_all(b".\r\n")
        .await
        .map_err(|e| format!("写入正文结束符失败: {e}"))?;
    session
        .writer
        .flush()
        .await
        .map_err(|e| format!("刷新写缓冲失败: {e}"))?;

    let queued = session.read_reply().await?;
    if queued.code != 250 {
        return Err(format!("邮件未被接受: {}", queued.text));
    }

    // QUIT 失败不影响「已收下」的结论
    let _ = session.command("QUIT", 221).await;
    Ok(queued.text)
}

/// 单次 SMTP 会话的两半
struct Session {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

/// 一条服务器应答
struct Reply {
    code: u16,
    text: String,
}

impl Session {
    /// 发一条命令并校验应答码
    async fn command(&mut self, line: &str, expected: u16) -> Result<String, String> {
        self.writer
            .write_all(format!("{line}\r\n").as_bytes())
            .await
            .map_err(|e| format!("发送命令 `{line}` 失败: {e}"))?;
        self.writer
            .flush()
            .await
            .map_err(|e| format!("刷新命令缓冲失败: {e}"))?;
        expect(self, expected, line).await
    }

    /// 发一条命令并接受多个合法应答码（`RCPT TO` 的 250/251 这类）
    async fn command_any(&mut self, line: &str, expected: &[u16]) -> Result<String, String> {
        self.writer
            .write_all(format!("{line}\r\n").as_bytes())
            .await
            .map_err(|e| format!("发送命令 `{line}` 失败: {e}"))?;
        self.writer
            .flush()
            .await
            .map_err(|e| format!("刷新命令缓冲失败: {e}"))?;
        let reply = self.read_reply().await?;
        if !expected.contains(&reply.code) {
            return Err(format!(
                "{line} 期望 {:?}，实际 {}: {}",
                expected, reply.code, reply.text
            ));
        }
        Ok(reply.text)
    }

    /// 读一条（可能多行的）应答
    async fn read_reply(&mut self) -> Result<Reply, String> {
        let mut text = String::new();
        loop {
            let mut line = String::new();
            let read = self
                .reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("读取服务器应答失败: {e}"))?;
            if read == 0 {
                return Err("SMTP 服务器关闭了连接".to_string());
            }
            let trimmed = line.trim_end_matches(['\r', '\n']);
            if text.is_empty() {
                text.push_str(trimmed);
            } else {
                text.push('\n');
                text.push_str(trimmed);
            }
            // `250-` 表示还有后续行，`250 ` 或 `250` 结束
            if trimmed.len() >= 4 && trimmed.as_bytes()[3] == b'-' {
                continue;
            }
            break;
        }
        let code = text
            .get(..3)
            .and_then(|prefix| prefix.parse::<u16>().ok())
            .ok_or_else(|| format!("无法解析 SMTP 应答: {text}"))?;
        Ok(Reply { code, text })
    }
}

/// 校验应答码，不符就带上服务器原文返回
async fn expect(session: &mut Session, expected: u16, stage: &str) -> Result<String, String> {
    let reply = session.read_reply().await?;
    if reply.code != expected {
        return Err(format!(
            "{stage} 期望 {expected}，实际 {}: {}",
            reply.code, reply.text
        ));
    }
    Ok(reply.text)
}

/// # SMTP 认证
///
/// 优先 `AUTH PLAIN`（一次往返），不支持再走 `AUTH LOGIN`（三次往返）。
/// 服务器 `EHLO` 里没列 AUTH 时直接失败——不去试探，避免把错误报文当成认证失败。
async fn authenticate(
    session: &mut Session,
    options: &EmailOptions,
    capabilities: &str,
) -> Result<(), String> {
    let upper = capabilities.to_uppercase();
    let plain = upper.contains("AUTH") && upper.contains("PLAIN");
    let login = upper.contains("AUTH") && upper.contains("LOGIN");

    if plain {
        let payload = format!("\0{}\0{}", options.username.trim(), options.password);
        let encoded = base64(payload.as_bytes());
        session
            .command(&format!("AUTH PLAIN {encoded}"), 235)
            .await?;
        return Ok(());
    }

    if login {
        let login_sent = session.command("AUTH LOGIN", 334).await?;
        let _ = login_sent;
        session
            .command(&base64(options.username.trim().as_bytes()), 334)
            .await?;
        session
            .command(&base64(options.password.as_bytes()), 235)
            .await?;
        return Ok(());
    }

    Err(
        "SMTP 服务器未提供 AUTH PLAIN/LOGIN，无法使用配置的账号（清空 username 可走匿名中继）"
            .to_string(),
    )
}

/// # base64 编码（标准字母表 + `=` 填充）
fn base64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);

        out.push(TABLE[(first >> 2) as usize] as char);
        out.push(TABLE[(((first & 0b0000_0011) << 4) | (second >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((second & 0b0000_1111) << 2) | (third >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(third & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// 邮箱打码：保留本地名首字符与域名，够定位人又不把完整地址写进日志
fn mask(address: &str) -> String {
    match address.split_once('@') {
        Some((local, domain)) => {
            let head: String = local.chars().take(1).collect();
            format!("{head}***@{domain}")
        }
        None => "***".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> DeliveryMsg {
        DeliveryMsg {
            persisted: true,
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
            channel_id: 5,
            channel_type: ChannelCode::Email.as_str().to_string(),
            address: Some("notice@example.com".to_string()),
        }
    }

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64("中文".as_bytes()), "5Lit5paH");
    }

    #[test]
    fn message_has_encoded_subject_and_body() {
        let options = EmailOptions {
            from_address: "noreply@example.com".to_string(),
            from_name: "消息中心".to_string(),
            body_template: "<h3>{{title}}</h3><p>{{content}}</p>".to_string(),
            content_type: "text/html".to_string(),
            ..Default::default()
        };
        let message = build_message(&options, &task(), "notice@example.com");

        assert!(message.contains("To: <notice@example.com>"));
        assert!(message.contains("From: =?UTF-8?B?"));
        assert!(message.contains("Content-Type: text/html; charset=UTF-8"));
        assert!(message.contains("Content-Transfer-Encoding: base64"));
        assert!(message.ends_with("\r\n"));

        // 正文部分解码后应含模板变量替换后的内容
        let body = message.split("\r\n\r\n").nth(1).expect("必须有头/体分隔");
        let body_b64: String = body.lines().collect();
        assert!(body_b64.starts_with("PGgzPg") || body_b64.starts_with("PGgzP"));
    }

    #[test]
    fn empty_templates_fall_back_to_task_fields() {
        let options = EmailOptions {
            from_address: "noreply@example.com".to_string(),
            ..Default::default()
        };
        let message = build_message(&options, &task(), "notice@example.com");
        let body = message.split("\r\n\r\n").nth(1).expect("必须有头/体分隔");
        let body_b64: String = body.lines().collect();
        // 「50碱区域有人停留超时」的 base64
        let expect = base64("50碱区域有人停留超时".as_bytes());
        assert!(body_b64.contains(&expect[..20]));
    }

    #[test]
    fn encoded_word_wraps_long_text() {
        let long = "很长的中文主题".repeat(10);
        let encoded = encoded_word(&long);
        assert!(encoded.starts_with("=?UTF-8?B?"));
        for line in encoded.split("\r\n ") {
            assert!(line.starts_with("=?UTF-8?B?") && line.ends_with("?="));
        }
    }

    #[test]
    fn email_format_is_rejected_when_dirty() {
        assert!(is_email("a@b.com"));
        assert!(is_email("notice@example.com"));
        assert!(!is_email("a b@c.com"));
        assert!(!is_email("a@b@c.com"));
        assert!(!is_email("a@"));
        assert!(!is_email("@b.com"));
        assert!(!is_email("no-at-sign"));
    }

    #[test]
    fn mask_keeps_domain() {
        assert_eq!(mask("notice@example.com"), "n***@example.com");
        assert_eq!(mask("bad-address"), "***");
    }

    #[test]
    fn body_lines_never_exceed_limit() {
        let encoded = base64(&vec![b'x'; 400]);
        let wrapped = wrap_base64(&encoded, BODY_LINE_LEN);
        for line in wrapped.split("\r\n") {
            assert!(line.len() <= BODY_LINE_LEN);
        }
    }
}
