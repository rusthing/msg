use crate::dto::*;
use crate::vo::{MsgMessageQueueExVo, MsgMessageQueueVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageQueueApiClient {
    client: FeignApiClient,
}

impl MsgMessageQueueApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
