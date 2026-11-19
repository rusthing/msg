use robotech::macros::crud_dto;

/// # 组织投影（由同步写入，禁止在 msg 侧修改）的写入 DTO
#[crud_dto]
pub struct MsgOrgDto {
    /// 组织编码
    /// adm 侧全局唯一；投影里不建 AK
    pub code: Option<String>,
    /// 树编码
    /// 字典序即树的前序；总长 = 层数 × 3
    /// @len 3..=30
    pub tree_code: String,
    /// 名称
    /// 兄弟同名常见，不做唯一
    pub name: String,
    /// 全称
    /// 可为空
    pub full_name: Option<String>,
    /// 是否启用
    /// false 时自身及整棵子树都不投递（连带）
    pub enabled: bool,
}
