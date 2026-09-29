use robotech::api_client::ApiClientError;
use robotech::mq::nats::NatsError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MsgPubError {
    #[error("发送消息到 NATS 失败: {0}")]
    Publish(#[from] NatsError),
    #[error("调用 msg-svr API 失败: {0}")]
    Api(#[from] ApiClientError),
}