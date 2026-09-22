use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: i64,
    /// 目标类别ID
    pub target_category_id: i64,
    /// 目标ID
    pub target_id: Option<i64>,
}