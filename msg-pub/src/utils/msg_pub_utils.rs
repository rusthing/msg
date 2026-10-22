//! # 消息发布核心逻辑
//!
//! 通过 msg-api-client 的 Feign 客户端调用 msg-svr 完成模板查询、渲染和投递创建，
//! 以及消息队列、事件来源的注册。

use crate::utils::msg_pub_error::MsgPubError;
use msg_api_client::cst::MSG_MESSAGE_TOPIC;
use msg_api_client::mqo::message_mqo::MessageMqo;
use robotech::mq::nats::publish;

/// # 发布模式
///
/// 决定「发布方是否等待服务端确认」，两者都是同一套消息体、同一个主题，
/// 差别只在可靠性语义和调用延迟。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PublishMode {
    /// **JetStream 持久化发布（等 ACK）** — 默认
    ///
    /// - 服务端确认落盘后才返回，`msg-svr` 停机期间消息留在 stream 里等恢复后补消费；
    /// - 发布失败（无 stream、连接断开、超时）会返回错误，调用方可重试或补偿；
    /// - 每条消息多一次往返延迟。
    ///
    /// 适合：业务事件、通知、任何「丢一条就是事故」的消息。
    #[default]
    Persistent,

    /// **Core NATS 尽力而为发布（不等 ACK）**
    ///
    /// - 发出即返回，不保证送达；连接正常时通常能到，服务端不可用则静默丢弃；
    /// - 注意：只要目标 subject 已被 JetStream stream 覆盖，服务端仍会照常落盘，
    ///   只是发布方不等 ACK、也就感知不到失败。
    ///
    /// 适合：高频、可丢、可被下一条覆盖的消息（如位置上报、心跳、状态刷新）。
    BestEffort,
}

/// # 发布一条消息到消息中心
///
/// 等价于 [`publish_message_with`] + [`PublishMode::Persistent`]：
/// 走 JetStream 持久化发布并等待 ACK，发布失败返回 [`MsgPubError`]，
/// 不再像裸 core publish 那样静默丢消息。
///
/// ## 注意事项
///
/// - 依赖 msg-svr 启动时创建的 stream（订阅 `MSG_MESSAGE_TOPIC` 的那个）。
///   首启顺序反过来（msg-svr 还没建 stream）时发布会报「no stream matches subject」，
///   这是**期望行为**：宁可报错，也不要无声丢弃。
/// - 高频、可丢的场景请用 [`publish_message_with`] + [`PublishMode::BestEffort`]，
///   不要为了统一入口付出每条一次的 ACK 往返。
pub async fn publish_message(message_mqo: MessageMqo) -> Result<(), MsgPubError> {
    publish_message_with(message_mqo, PublishMode::Persistent).await
}

/// # 按指定模式发布一条消息到消息中心
///
/// 两种模式的差异见 [`PublishMode`]。消息体与主题完全一致，
/// 调用方可以按业务对可靠性的要求自行选择，不需要绕过 msg-pub 直连 NATS。
pub async fn publish_message_with(
    message_mqo: MessageMqo,
    mode: PublishMode,
) -> Result<(), MsgPubError> {
    publish_message_to(MSG_MESSAGE_TOPIC, message_mqo, mode).await
}

/// # 发布一条消息到指定队列
///
/// 与 [`publish_message_with`] 的唯一差别是目标 subject 由调用方给出。业务方
/// 有自己的消息入口队列时（已通过 `msg_message_queue` 注册给消息中心），用
/// 注册的那个 `code` 发；两者必须是同一个 subject，否则「注册的队列没人发，
/// 发的 subject 没人订阅」。
pub async fn publish_message_to(
    subject: &str,
    message_mqo: MessageMqo,
    mode: PublishMode,
) -> Result<(), MsgPubError> {
    let persistent = matches!(mode, PublishMode::Persistent);
    publish(subject, &message_mqo, persistent).await?;
    Ok(())
}
