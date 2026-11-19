use robotech::macros::vo;

/// # 人员投影（由同步写入，禁止在 msg 侧修改）的查询 VO
#[vo]
pub struct MsgPersonVo {
    /// ID
    /// 同 adm_person.id
    pub id: u64,
    /// 手机号
    /// user.mobile 为空时的短信兜底地址
    pub mobile: Option<String>,
    /// 邮箱
    /// user.email 为空时的邮件兜底地址
    pub email: Option<String>,
    /// 是否启用
    /// false 不阻断投递：地址照取，只做可观测
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
