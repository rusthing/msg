use crate::{
    MsgChannelApiClient, MsgDeliveryApiClient, MsgDeliveryChannelApiClient,
    MsgDeliveryChannelLogApiClient, MsgDeliveryTargetApiClient, MsgEventSourceApiClient,
    MsgMessageApiClient, MsgMessageCategoryApiClient, MsgMessageQueueApiClient,
    MsgMessageTargetApiClient, MsgTargetCategoryApiClient, MsgTargetCategoryChannelApiClient,
};
use robotech::macros::api_client;

#[api_client]
pub struct MsgApiClient {
    pub channel: MsgChannelApiClient,
    pub delivery: MsgDeliveryApiClient,
    pub delivery_channel: MsgDeliveryChannelApiClient,
    pub delivery_channel_log: MsgDeliveryChannelLogApiClient,
    pub delivery_target: MsgDeliveryTargetApiClient,
    pub event_source: MsgEventSourceApiClient,
    pub message: MsgMessageApiClient,
    pub message_category: MsgMessageCategoryApiClient,
    pub message_queue: MsgMessageQueueApiClient,
    pub message_target: MsgMessageTargetApiClient,
    pub target_category: MsgTargetCategoryApiClient,
    pub target_category_channel: MsgTargetCategoryChannelApiClient,
}