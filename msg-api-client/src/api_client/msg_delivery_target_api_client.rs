use crate::dto::*;
use crate::vo::{MsgDeliveryTargetExVo, MsgDeliveryTargetVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgDeliveryTargetApiClient {
    client: FeignApiClient,
}

impl MsgDeliveryTargetApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
