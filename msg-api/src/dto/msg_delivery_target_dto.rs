use crate::dic::DeliverTargetStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryTargetDto {
    pub delivery_id: u64,
    pub target_category_id: u64,
    pub target_id: u64,
    #[db_default]
    pub deliver_target_status: DeliverTargetStatus,
}