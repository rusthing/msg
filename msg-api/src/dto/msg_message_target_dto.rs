use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID）
    ///
    /// 大屏这类动态接入的目标填 0，表示该类别下的全部目标：发布方没法在
    /// 登记时知道有哪些屏，只能声明「发给大屏这一类」。
    pub target_id: u64,
}