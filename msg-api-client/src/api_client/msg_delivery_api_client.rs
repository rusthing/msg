use crate::dto::*;
use crate::vo::{MsgDeliveryExVo, MsgDeliveryVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgDeliveryApiClient {
    client: FeignApiClient,
}

impl MsgDeliveryApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
