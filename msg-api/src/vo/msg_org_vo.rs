use robotech::macros::vo;

/// # 组织投影（由同步写入，禁止在 msg 侧修改）的查询 VO
#[vo]
pub struct MsgOrgVo {
    /// ID
    /// 同 adm_org.id
    pub id: u64,
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
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    /// @unit ms
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 源行更新时间戳
    /// 取自 adm 侧同名表，不是同步时间
    /// @unit ms
    pub update_ms: u64,
}
