use crate::dto::*;
use crate::vo::{MsgPublishQueueExVo, MsgPublishQueueVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgPublishQueueApiClient {
    client: FeignApiClient,
}

impl MsgPublishQueueApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
