use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageLevelDto {
    /// 编码，用于消息队列的名称
    pub code: String,
    /// 名称
    pub name: String,
    /// 备注
    pub remark: Option<String>,
}