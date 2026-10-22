use crate::dto::*;
use crate::vo::{MsgMessageLevelExVo, MsgMessageLevelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageLevelApiClient {
    client: FeignApiClient,
}

impl MsgMessageLevelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
