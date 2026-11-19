use robotech::macros::crud_dto;

/// # 角色用户投影（由同步写入，禁止在 msg 侧修改）的写入 DTO
#[crud_dto]
pub struct MsgRoleUserDto {
    /// 角色ID
    /// 关联 msg_role.id
    pub role_id: u64,
    /// 用户ID
    /// 关联 msg_user.id
    pub user_id: u64,
}
