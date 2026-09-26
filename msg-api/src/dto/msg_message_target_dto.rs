use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    pub message_id: i64,
    pub target_category_id: i64,
    pub target_id: i64,
}