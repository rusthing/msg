use robotech::macros::vo;

#[vo]
pub struct MsgMessageQueueVo {
    /// ID
    pub id: u64,
    /// 编码，用于订阅消息中间件队列的名称
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化
    pub persisted: bool,
    /// 备注
    pub remark: Option<String>,
    /// 创建者ID
    pub creator_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updator_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}