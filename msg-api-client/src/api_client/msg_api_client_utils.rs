use crate::{
    MsgChannelApiClient, MsgDeliveryApiClient, MsgDeliveryChannelApiClient,
    MsgDeliveryChannelLogApiClient, MsgDeliveryTargetApiClient, MsgEventSourceApiClient,
    MsgMessageApiClient, MsgMessageCategoryApiClient, MsgMessageQueueApiClient,
    MsgMessageTargetApiClient, MsgTargetCategoryChannelApiClient,
};
use robotech::macros::api_client;

#[api_client]
pub struct MsgApiClient {
    pub channel_client: MsgChannelApiClient,
    pub delivery_client: MsgDeliveryApiClient,
    pub delivery_channel_client: MsgDeliveryChannelApiClient,
    pub delivery_channel_log_client: MsgDeliveryChannelLogApiClient,
    pub delivery_target_client: MsgDeliveryTargetApiClient,
    pub event_source_client: MsgEventSourceApiClient,
    pub message_client: MsgMessageApiClient,
    pub message_category_client: MsgMessageCategoryApiClient,
    pub message_queue_client: MsgMessageQueueApiClient,
    pub message_target_client: MsgMessageTargetApiClient,
    pub target_category_client: MsgMessageTargetApiClient,
    pub target_category_channel_client: MsgTargetCategoryChannelApiClient,
}