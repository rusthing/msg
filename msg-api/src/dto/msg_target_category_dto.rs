use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryDto {
    /// 编码
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
    /// 是否允许广播：false 表示必须给出具体 target_id
    pub allow_broadcast: bool,
    /// 目标标识键：从消息 labels 取目标标识的键，逗号分隔按序取第一个有值者
    /// @len 0..=100
    pub target_keys: Option<String>,
}
