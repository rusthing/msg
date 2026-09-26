use crate::dic::DeliverTargetStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryTargetVo {
    pub id: i64,
    pub delivery_id: i64,
    pub target_category_id: i64,
    pub target_id: i64,
    pub deliver_target_status: DeliverTargetStatus,
    pub creator_id: i64,
    pub create_ms: i64,
    pub updator_id: i64,
    pub update_ms: i64,
}