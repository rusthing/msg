use crate::dic::DeliverTargetStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryTargetDto {
    /// 投递ID
    pub delivery_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID）
    pub target_id: u64,
    /// 投递状态（0-投递中、1-投递成功、2-投递失败、3-目标已读）
    #[db_default]
    pub deliver_target_status: DeliverTargetStatus,
}