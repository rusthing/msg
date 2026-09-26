use crate::dto::*;
use crate::vo::{MsgEventSourceExVo, MsgEventSourceVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgEventSourceApiClient {
    client: FeignApiClient,
}

impl MsgEventSourceApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}