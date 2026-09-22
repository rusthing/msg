use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageChannelDto {
    /// 消息ID
    pub message_id: i64,
    /// 渠道ID
    pub channel_id: i64,
}