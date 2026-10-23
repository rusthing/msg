/// 消息主题
pub const MSG_MESSAGE_TOPIC: &str = "msg.message";
/// 投递主题前缀
///
/// 实际主题为 `{前缀}.{渠道编码}`，例如 `msg.delivery.websocket`。
/// 一个渠道一个主题，新增渠道只增加订阅方，投递侧不用改。
pub const MSG_DELIVERY_TOPIC: &str = "msg.delivery";

/// websocket 渠道编码
///
/// 同时是 `msg_channel.code`、投递主题末段（`msg.delivery.websocket`）
/// 与 `ChannelSender::code` 的取值，三处必须一致。
pub const MSG_CHANNEL_CODE_WEBSOCKET: &str = "websocket";

/// # 拼出某个渠道的投递主题
///
/// 渠道编码来自 `msg_channel.code`，会直接拼进 NATS subject，因此必须只含
/// `[A-Za-z0-9_-]`（NATS 以 `.` 分段、不允许 `*` `>` 等通配符）。
pub fn delivery_subject(channel_code: &str) -> String {
    format!("{MSG_DELIVERY_TOPIC}.{channel_code}")
}
