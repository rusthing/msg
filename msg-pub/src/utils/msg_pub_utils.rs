//! # 消息发布核心逻辑
//!
//! 通过 msg-api-client 的 Feign 客户端调用 msg-svr 完成模板查询、渲染和投递创建。

use crate::utils::msg_pub_error::MsgPubError;
use msg_api::cst::MSG_MESSAGE_TOPIC;
use msg_api::mqo::message_mqo::MessageMqo;
use robotech::mq::nats::publish;

pub async fn publish_message(message_mqo: MessageMqo) -> Result<(), MsgPubError> {
    publish(MSG_MESSAGE_TOPIC, &message_mqo, false).await?;
    Ok(())
}
