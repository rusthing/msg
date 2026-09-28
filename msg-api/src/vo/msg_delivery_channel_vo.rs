use crate::dic::DeliverChannelStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryChannelVo {
    /// ID
    pub id: u64,
    /// 投递目标ID
    pub deliver_target_id: u64,
    /// 渠道ID
    pub channel_id: u64,
    /// 投递状态
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    pub address: Option<String>,
    /// 创建者ID
    pub creator_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updator_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}