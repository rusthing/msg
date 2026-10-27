use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageQueueDto {
    /// 编码，消息主题，订阅与发布都用它
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 是否持久化
    /// 持久化走 JetStream（发布方等 ACK，服务重启后补消费）；不持久化走 Core NATS，发出即不管送达
    #[db_default]
    pub persisted: bool,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
}