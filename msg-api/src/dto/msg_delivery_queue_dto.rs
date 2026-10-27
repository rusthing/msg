use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryQueueDto {
    /// 编码：投递主题，固定为 `msg.delivery.{渠道编码}`
    ///
    /// 末段必须等于某个 `msg_channel.code`，渠道编码从主题反解，本表不存渠道 ID。
    /// 写入时校验前缀与渠道存在，见 `MsgDeliveryQueueSvc` 的 before 钩子。
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
