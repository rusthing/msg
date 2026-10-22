use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageCategoryDto {
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 备注
    pub remark: Option<String>,
}