use robotech::macros::crud_dto;

/// # 人员投影（由同步写入，禁止在 msg 侧修改）的写入 DTO
#[crud_dto]
pub struct MsgPersonDto {
    /// 手机号
    /// user.mobile 为空时的短信兜底地址
    pub mobile: Option<String>,
    /// 邮箱
    /// user.email 为空时的邮件兜底地址
    pub email: Option<String>,
    /// 是否启用
    /// false 不阻断投递：地址照取，只做可观测
    pub enabled: bool,
}
