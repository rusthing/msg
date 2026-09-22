use crate::dic::DeliverTargetStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryTargetVo {
    /// ID
    pub id: i64,
    /// 投递ID
    pub delivery_id: i64,
    /// 目标类别ID
    pub target_category_id: i64,
    /// 目标ID
    pub target_id: Option<i64>,
    /// 投递状态
    pub deliver_target_status: DeliverTargetStatus,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}