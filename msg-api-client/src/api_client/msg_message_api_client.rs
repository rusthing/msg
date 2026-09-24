use crate::dto::*;
use crate::vo::{MsgMessageExVo, MsgMessageVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageApiClient {
    client: FeignApiClient,
}

impl MsgMessageApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
