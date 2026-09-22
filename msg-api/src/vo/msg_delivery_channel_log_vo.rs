use crate::dic::DeliverChannelStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryChannelLogVo {
    /// ID
    pub id: i64,
    /// 投递渠道ID
    pub delivery_channel_id: i64,
    /// 投递状态
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    pub address: Option<String>,
    /// 投递详情
    pub detail: Option<String>,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}