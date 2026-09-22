use crate::dic::DeliverTargetStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryTargetDto {
    /// 投递ID
    pub delivery_id: i64,
    /// 目标类别ID
    pub target_category_id: i64,
    /// 目标ID
    pub target_id: Option<i64>,
    /// 投递状态
    #[db_default]
    pub deliver_target_status: DeliverTargetStatus,
}