use robotech::mq::nats::NatsError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MsgPubError {
    #[error("发送消息到 NATS 失败: {0}")]
    Publish(#[from] NatsError),
}
