use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageCategoryDto {
    /// 编码
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
}