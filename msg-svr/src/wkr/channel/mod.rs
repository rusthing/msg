//! # 渠道发送器
//!
//! 每种渠道实现一个 [`ChannelSender`]，按 `channel_code` 分发。
//!
//! ## 扩展方式
//!
//! 接入短信/邮箱时：
//!
//! 1. 在 `msg_channel` 里加一条渠道，`code` 只用 `[A-Za-z0-9_-]`；
//! 2. 实现一个 `ChannelSender`，`code()` 返回同样的值；
//! 3. 在 [`init_senders`] 里注册；
//! 4. 为该渠道的投递主题（`msg.delivery.{code}`）起订阅。
//!
//! 投递侧（`msg-svr` 发任务的地方）不需要任何改动——它只按渠道表里的 code
//! 发到对应主题。

pub mod websocket;

use async_trait::async_trait;
use msg_api::msg::DeliveryMsg;
use std::sync::{Arc, OnceLock};

/// # 单次发送结果
#[derive(Debug, Clone)]
pub enum SendOutcome {
    /// 已写入连接/网关，附带落库用的地址
    Delivered { address: String },
    /// 目标当前不在线（websocket 特有：没有连接就没有必要重试）
    NoConnection,
    /// 发送失败
    Failed(String),
}

impl SendOutcome {
    /// 是否算发送成功
    pub fn is_delivered(&self) -> bool {
        matches!(self, Self::Delivered { .. })
    }

    /// 落库用的地址
    pub fn address(&self) -> Option<String> {
        match self {
            Self::Delivered { address } => Some(address.clone()),
            _ => None,
        }
    }

    /// 落库用的详情
    pub fn detail(&self) -> &str {
        match self {
            Self::Delivered { .. } => "发送成功",
            Self::NoConnection => "目标未连接",
            Self::Failed(reason) => reason,
        }
    }
}

/// # 渠道发送器
#[async_trait]
pub trait ChannelSender: Send + Sync {
    /// 渠道编码，与 `msg_channel.code`、投递主题末段一致
    fn code(&self) -> &'static str;
    /// 发送一条投递任务
    async fn send(&self, task: &DeliveryMsg) -> SendOutcome;
}

static SENDERS: OnceLock<Vec<Arc<dyn ChannelSender>>> = OnceLock::new();

/// # 注册所有已实现的渠道
///
/// 幂等：重复调用不会重复注册（`OnceLock` 只认第一次）。
pub fn init_senders() {
    let _ = SENDERS.set(vec![Arc::new(websocket::WebSocketSender)]);
}

/// # 按渠道分发一条任务
///
/// 渠道没实现时返回 [`SendOutcome::Failed`]，不会静默丢弃——
/// 否则运营配了新渠道却一直没人发，很难查。
pub async fn dispatch(task: &DeliveryMsg) -> SendOutcome {
    let senders = SENDERS.get().map(Vec::as_slice).unwrap_or(&[]);
    match senders.iter().find(|s| s.code() == task.channel_code) {
        Some(sender) => sender.send(task).await,
        None => SendOutcome::Failed(format!("渠道未实现: {}", task.channel_code)),
    }
}
