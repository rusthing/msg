use robotech::macros::vo;

/// # 角色投影（由同步写入，禁止在 msg 侧修改）的查询 VO
#[vo]
pub struct MsgRoleVo {
    /// ID
    /// 同 adm_role.id
    pub id: u64,
    /// 角色分组ID
    /// 只在页面按分组展示时用，不参与投递
    pub group_id: u64,
    /// 名称
    /// 同一分组内唯一，投影里不建 AK
    pub name: String,
    /// 是否启用
    /// false 时不投递该角色
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
