//! # 内置渠道类型
//!
//! `msg_channel.channel_type` 在库里是字符串：运营可以新增未登记的类型，配一条投递队列就
//! 有订阅，再注册一个发送器就有人发。所以枚举只覆盖**代码里实现过发送器的类型**，
//! 作为常量、发送器与前端配置表单的唯一事实来源；从库里读出来的未知 code 仍然
//! 按字符串透传（[`ChannelCode::from_code`] 返回 `None`），不强迫扩展渠道先改这里。
//!
//! 新增一种内置渠道的顺序：
//!
//! 1. 这里加一个变体（`as_str` / `from_code` / `ALL` 三处）；
//! 2. 在 `msg-svr` 里实现 `ChannelSender`，`channel()` 返回新变体；
//! 3. 在 `msg_delivery_queue` 里配投递队列 `msg.delivery.{channel_type}`。

use std::fmt;

/// 内置渠道类型
///
/// 取值与 `msg_channel.channel_type`、投递主题末段（`msg.delivery.{channel_type}`）、
/// `ChannelSender::channel()` 三处一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelCode {
    /// WebSocket 长连接（大屏）
    Websocket,
    /// Core NATS 主题（主题与报文帧由注册渠道的一方写在 options / 载荷模板里）
    Nats,
    /// 短信网关
    Sms,
    /// SMTP 邮件
    Email,
}

impl ChannelCode {
    /// 所有内置类型，按编码字典序排列
    pub const ALL: [ChannelCode; 4] = [
        ChannelCode::Email,
        ChannelCode::Nats,
        ChannelCode::Sms,
        ChannelCode::Websocket,
    ];

    /// 渠道类型（`msg_channel.channel_type`，同时也是投递主题末段）
    pub const fn as_str(self) -> &'static str {
        match self {
            ChannelCode::Email => "email",
            ChannelCode::Nats => "nats",
            ChannelCode::Sms => "sms",
            ChannelCode::Websocket => "websocket",
        }
    }

    /// 编码 → 内置类型；库里配的其它 code 返回 `None`
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|channel| channel.as_str() == code)
    }
}

impl fmt::Display for ChannelCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_round_trip() {
        for channel in ChannelCode::ALL {
            assert_eq!(ChannelCode::from_code(channel.as_str()), Some(channel));
        }
    }

    #[test]
    fn unknown_code_is_none() {
        assert_eq!(ChannelCode::from_code("locator"), None);
        assert_eq!(ChannelCode::from_code(""), None);
    }

    /// 新增类型时把这里改成新数量：提醒人别只加变体忘了 ALL
    #[test]
    fn all_covers_every_variant() {
        assert_eq!(ChannelCode::ALL.len(), 4);
    }
}
