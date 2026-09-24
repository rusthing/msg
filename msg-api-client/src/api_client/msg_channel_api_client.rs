use crate::dto::*;
use crate::vo::{MsgChannelExVo, MsgChannelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgChannelApiClient {
    client: FeignApiClient,
}

impl MsgChannelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
