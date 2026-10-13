use crate::dic::DeliverTargetStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryTargetVo {
    pub id: u64,
    pub delivery_id: u64,
    pub target_category_id: u64,
    pub target_id: u64,
    pub deliver_target_status: DeliverTargetStatus,
    pub creater_id: u64,
    pub create_ms: u64,
    pub updater_id: u64,
    pub update_ms: u64,
}