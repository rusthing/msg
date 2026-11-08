use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeadLetterDto {
    /// 原消息主题
    /// @len 1..=150
    pub subject: String,
    /// 转入死信的原因
    /// @len 1..
    pub reason: String,
    /// 原始消息负载
    /// @len 1..
    pub payload: String,
    /// 已经投递的次数
    #[db_default]
    pub delivered: u64,
    /// 是否已人工处理
    #[db_default]
    pub handled: bool,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
    /// 是否模拟（原始负载带 simulated=true 或 source=sim 标记时为是）
    #[db_default]
    pub simulated: bool,
}
