use crate::dto::*;
use crate::vo::{MsgMessageChannelExVo, MsgMessageChannelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageChannelApiClient {
    client: FeignApiClient,
}

impl MsgMessageChannelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
