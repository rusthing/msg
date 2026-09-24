use crate::dto::*;
use crate::vo::{MsgDeliveryChannelLogExVo, MsgDeliveryChannelLogVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgDeliveryChannelLogApiClient {
    client: FeignApiClient,
}

impl MsgDeliveryChannelLogApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
