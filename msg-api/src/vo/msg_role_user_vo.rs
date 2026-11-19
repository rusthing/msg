use robotech::macros::vo;

/// # 角色用户投影（由同步写入，禁止在 msg 侧修改）的查询 VO
#[vo]
pub struct MsgRoleUserVo {
    /// ID
    /// 同 adm_role_user.id
    pub id: u64,
    /// 角色ID
    /// 关联 msg_role.id
    pub role_id: u64,
    /// 用户ID
    /// 关联 msg_user.id
    pub user_id: u64,
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
