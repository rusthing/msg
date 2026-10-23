use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryQueueVo {
    /// ID
    pub id: u64,
    /// 渠道ID（一个渠道一条投递队列）
    pub channel_id: u64,
    /// 编码：投递主题，订阅与投递都用它
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化（持久化走 JetStream，不持久化走 Core NATS 广播）
    pub persisted: bool,
    /// 备注
    pub remark: Option<String>,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}
