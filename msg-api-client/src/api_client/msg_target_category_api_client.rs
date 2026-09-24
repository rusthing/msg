use crate::dto::*;
use crate::vo::{MsgTargetCategoryExVo, MsgTargetCategoryVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgTargetCategoryApiClient {
    client: FeignApiClient,
}

impl MsgTargetCategoryApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
