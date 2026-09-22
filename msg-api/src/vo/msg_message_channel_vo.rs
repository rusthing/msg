use robotech::macros::vo;

#[vo]
pub struct MsgMessageChannelVo {
    /// ID
    pub id: i64,
    /// 消息ID
    pub message_id: i64,
    /// 渠道ID
    pub channel_id: i64,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}