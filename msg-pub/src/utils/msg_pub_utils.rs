//! # 消息发布核心逻辑
//!
//! 通过 msg-api-client 的 Feign 客户端调用 msg-svr 完成模板查询、渲染和投递创建，
//! 以及消息队列、事件来源、消息类别的注册。

use crate::config::get_api_config;
use crate::utils::msg_pub_error::MsgPubError;
use msg_api::cst::MSG_MESSAGE_TOPIC;
use msg_api::mqo::message_mqo::MessageMqo;
use robotech::api::U64;
use robotech::micro_svc::FeignApiClient;
use robotech::mq::nats::publish;

pub async fn publish_message(message_mqo: MessageMqo) -> Result<(), MsgPubError> {
    publish(MSG_MESSAGE_TOPIC, &message_mqo, false).await?;
    Ok(())
}
