use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标标识（用户 / 角色 / 分组 / 大屏的业务标识，最长 100 字符）
    ///
    /// 空串表示该类别下的全部目标：大屏这类动态接入的目标，发布方没法在
    /// 登记时知道有哪些屏，只能声明「发给大屏这一类」。
    /// @len 0..=100
    pub target_id: String,
}
