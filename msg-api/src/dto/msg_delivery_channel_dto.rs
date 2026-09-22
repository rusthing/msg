use crate::dic::DeliverChannelStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryChannelDto {
    /// 投递目标ID
    pub deliver_target_id: i64,
    /// 渠道ID
    pub channel_id: i64,
    /// 投递状态
    #[db_default]
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    pub address: Option<String>,
}