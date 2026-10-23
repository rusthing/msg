use crate::dto::*;
use crate::vo::{MsgDeliveryQueueExVo, MsgDeliveryQueueVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgDeliveryQueueApiClient {
    client: FeignApiClient,
}

impl MsgDeliveryQueueApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
