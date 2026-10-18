use robotech::macros::vo;

#[vo]
pub struct MsgDeadLetterVo {
    /// ID
    pub id: u64,
    /// 原消息主题
    pub subject: String,
    /// 转入死信的原因
    pub reason: String,
    /// 原始消息负载
    pub payload: String,
    /// 已经投递的次数
    pub delivered: u64,
    /// 是否已人工处理
    pub handled: bool,
    /// 备注
    pub remark: Option<String>,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}
