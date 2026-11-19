use crate::dto::*;
use crate::vo::{MsgMessageTargetChannelExVo, MsgMessageTargetChannelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageTargetChannelApiClient {
    client: FeignApiClient,
}

impl MsgMessageTargetChannelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
