use robotech::macros::vo;

#[vo]
pub struct MsgMessageQueueVo {
    /// ID
    pub id: i64,
    /// 编码，用于订阅消息中间件队列的名称
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化
    pub persisted: bool,
    /// 备注
    pub remark: Option<String>,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}