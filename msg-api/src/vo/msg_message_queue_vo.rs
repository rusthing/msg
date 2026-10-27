use robotech::macros::vo;

#[vo]
pub struct MsgMessageQueueVo {
    /// ID
    pub id: u64,
    /// 编码，消息主题，订阅与发布都用它
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化
    /// 持久化走 JetStream（发布方等 ACK，服务重启后补消费）；不持久化走 Core NATS，发出即不管送达
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