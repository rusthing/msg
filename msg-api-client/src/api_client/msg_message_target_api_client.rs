use crate::dto::*;
use crate::vo::{MsgMessageTargetExVo, MsgMessageTargetVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageTargetApiClient {
    client: FeignApiClient,
}

impl MsgMessageTargetApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
