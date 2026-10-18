use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeadLetterDto {
    /// 原消息主题
    pub subject: String,
    /// 转入死信的原因
    pub reason: String,
    /// 原始消息负载
    pub payload: String,
    /// 已经投递的次数
    #[db_default]
    pub delivered: u64,
    /// 是否已人工处理
    #[db_default]
    pub handled: bool,
    /// 备注
    pub remark: Option<String>,
}
