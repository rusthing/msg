use crate::dto::*;
use crate::vo::{MsgTargetCategoryChannelExVo, MsgTargetCategoryChannelVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgTargetCategoryChannelApiClient {
    client: FeignApiClient,
}

impl MsgTargetCategoryChannelApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}