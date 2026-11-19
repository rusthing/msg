use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetChannelDto {
    /// 消息目标ID
    pub message_target_id: u64,
    /// 渠道ID
    pub channel_id: u64,
}
