use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID）
    ///
    /// 为空表示该目标类别下的全部目标，即按类别广播：大屏这类动态接入的
    /// 目标没法逐个登记，只能声明「发给大屏这一类」。
    pub target_id: Option<u64>,
}