use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryDto {
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 备注
    pub remark: Option<String>,
}