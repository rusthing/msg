use crate::dto::*;
use crate::vo::{MsgMessageSourceExVo, MsgMessageSourceVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageSourceApiClient {
    client: FeignApiClient,
}

impl MsgMessageSourceApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
