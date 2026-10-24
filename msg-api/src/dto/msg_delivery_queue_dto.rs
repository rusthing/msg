use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryQueueDto {
    /// 渠道ID（一个渠道一条投递队列）
    pub channel_id: u64,
    /// 编码：投递主题，订阅与投递都用它
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 是否持久化（持久化走 JetStream，不持久化走 Core NATS 广播）
    #[db_default]
    pub persisted: bool,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
}
