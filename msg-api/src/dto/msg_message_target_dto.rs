use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    pub message_id: u64,
    pub target_category_id: u64,
    pub target_id: u64,
}