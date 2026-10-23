use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID）
    pub target_id: u64,
}