//! # 渠道发送器
//!
//! 每种内置渠道实现一个 [`ChannelSender`]，按 [`ChannelCode`] 分发。
//!
//! ## 扩展方式
//!
//! 1. 在 `msg-api` 的 `ChannelCode` 里加一个变体（`msg_channel.channel_type` 用同一个值，
//!    只用 `[A-Za-z0-9_-]`）；
//! 2. 实现一个 `ChannelSender`，`channel()` 返回这个变体；
//! 3. 在 [`init_senders`] 里注册；
//! 4. 在 `msg_delivery_queue` 里为这个渠道配一条投递队列（主题 + 是否持久化），
//!    执行器按表自动建立订阅。
//!
//! 投递侧（`msg-svr` 发任务的地方）不需要任何改动——它按渠道的投递队列配置
//! 发到对应主题。
//!
//! 库里配了 `ChannelCode` 之外的新 code 也不会走错路：`dispatch` 会明确返回
//! 「渠道未实现」并打 error 日志，而不是静默丢弃。

pub mod email;
pub mod nats;
pub mod sms;
pub mod websocket;

use async_trait::async_trait;
use msg_api::cst::ChannelCode;
use msg_api::msg::DeliveryMsg;
use std::sync::{Arc, OnceLock};
use tracing::error;

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
    /// 发送器负责的渠道类型（`msg_channel.channel_type`）
    fn channel(&self) -> ChannelCode;
    /// 发送一条投递任务
    async fn send(&self, task: &DeliveryMsg) -> SendOutcome;
}

static SENDERS: OnceLock<Vec<Arc<dyn ChannelSender>>> = OnceLock::new();

/// # 注册所有已实现的渠道
///
/// 幂等：重复调用不会重复注册（`OnceLock` 只认第一次）。
pub fn init_senders() {
    let _ = SENDERS.set(vec![
        Arc::new(websocket::WebSocketSender),
        Arc::new(nats::NatsSender),
        Arc::new(sms::SmsSender),
        Arc::new(email::EmailSender),
    ]);
}

/// # 按渠道类型分发一条任务
///
/// 发送器按 `channel_type` 选：一个类型一条渠道，具体行为（subject / 网关 /
/// 发件账号）由这条渠道的 `options` 决定。
///
/// 渠道类型没实现时返回 [`SendOutcome::Failed`]，不会静默丢弃——
/// 否则运营配了新渠道却一直没人发，很难查。
pub async fn dispatch(task: &DeliveryMsg) -> SendOutcome {
    let senders = SENDERS.get().map(Vec::as_slice).unwrap_or(&[]);
    let Some(channel) = ChannelCode::from_code(&task.channel_type) else {
        error!(
            "渠道类型未实现（请在 ChannelCode 里登记并注册发送器）: channel_type={}, event_code={}",
            task.channel_type, task.event_code
        );
        return SendOutcome::Failed(format!("渠道未实现: {}", task.channel_type));
    };
    match senders.iter().find(|sender| sender.channel() == channel) {
        Some(sender) => sender.send(task).await,
        None => {
            error!(
                "渠道类型已登记但没注册发送器: channel={channel}, channel_type={}, event_code={}",
                task.channel_type, task.event_code
            );
            SendOutcome::Failed(format!("渠道未实现: {channel}"))
        }
    }
}
