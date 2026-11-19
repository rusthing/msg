use robotech::macros::crud_dto;

/// # 角色投影（由同步写入，禁止在 msg 侧修改）的写入 DTO
#[crud_dto]
pub struct MsgRoleDto {
    /// 角色分组ID
    /// 只在页面按分组展示时用，不参与投递
    pub group_id: u64,
    /// 名称
    /// 同一分组内唯一，投影里不建 AK
    pub name: String,
    /// 是否启用
    /// false 时不投递该角色
    pub enabled: bool,
}
