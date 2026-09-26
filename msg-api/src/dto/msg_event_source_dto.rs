use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgEventSourceDto {
    pub code: String,
    pub name: String,
    pub remark: Option<String>,
}