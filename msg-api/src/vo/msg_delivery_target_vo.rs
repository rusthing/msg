use crate::dic::DeliverTargetStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryTargetVo {
    /// ID
    pub id: u64,
    /// 投递ID
    pub delivery_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID），为空表示该类别下的全部目标
    pub target_id: Option<u64>,
    /// 投递状态（0-投递中、1-投递成功、2-投递失败、3-目标已读）
    pub deliver_target_status: DeliverTargetStatus,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}