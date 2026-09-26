use crate::dic::DeliverTargetStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryTargetDto {
    pub delivery_id: i64,
    pub target_category_id: i64,
    pub target_id: i64,
    #[db_default]
    pub deliver_target_status: DeliverTargetStatus,
}