use crate::dic::DeliverChannelStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryChannelLogDto {
    /// 投递渠道ID
    pub delivery_channel_id: u64,
    /// 投递状态
    #[db_default]
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    /// @len 0..=80
    pub address: Option<String>,
    /// 投递详情
    /// @len 0..
    pub detail: Option<String>,
}