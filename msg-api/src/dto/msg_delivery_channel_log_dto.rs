use crate::dic::DeliverChannelStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryChannelLogDto {
    /// 投递渠道ID
    pub delivery_channel_id: i64,
    /// 投递状态
    #[db_default]
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    pub address: Option<String>,
    /// 投递详情
    pub detail: Option<String>,
}