use crate::dto::*;
use crate::vo::{MsgDeliveryChannelExVo, MsgDeliveryChannelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgDeliveryChannelApiClient {
    client: FeignApiClient,
}

impl MsgDeliveryChannelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
