use crate::dic::DeliverTargetStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryTargetDto {
    /// 投递ID
    pub delivery_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标标识（用户 / 角色 / 分组 / 大屏的业务标识，最长 100 字符）
    ///
    /// 空串表示这次投递覆盖的是该类别下的全部目标（按类别广播），
    /// 没有具体目标可记。
    /// @len 0..=100
    pub target_id: String,
    /// 投递状态（0-投递中、1-投递成功、2-投递失败、3-目标已读）
    #[db_default]
    pub deliver_target_status: DeliverTargetStatus,
}
