use robotech::macros::crud_dto;

/// # 用户投影（由同步写入，禁止在 msg 侧修改）的写入 DTO
#[crud_dto]
pub struct MsgUserDto {
    /// 人员ID
    /// 关联 msg_person；为空表示纯账号，地址兜底取不到
    pub person_id: Option<u64>,
    /// 登录名
    /// 称呼兜底链的最末一级
    pub name: String,
    /// 昵称
    /// 称呼兜底链的第二级
    pub nick_name: Option<String>,
    /// 实名
    /// 称呼兜底链的第一级
    pub real_name: Option<String>,
    /// 手机号
    /// 短信首选；为空降级取 msg_person.mobile
    pub mobile: Option<String>,
    /// 邮箱
    /// 邮件首选；为空降级取 msg_person.email
    pub email: Option<String>,
    /// 过期时间戳
    /// 为空表示不过期；与 enabled 一起决定可达性
    /// @unit ms
    pub expire_ms: Option<u64>,
    /// 是否启用
    /// false 仍参与展开，只是不投递
    pub enabled: bool,
}
