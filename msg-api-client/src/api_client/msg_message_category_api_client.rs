use crate::dto::*;
use crate::vo::{MsgMessageCategoryExVo, MsgMessageCategoryVo};
use robotech::macros::feign;
use robotech::micro_svc::FeignApiClient;

#[feign]
pub struct MsgMessageCategoryApiClient {
    client: FeignApiClient,
}

impl MsgMessageCategoryApiClient {
    pub fn new(client: FeignApiClient) -> Self {
        Self { client }
    }
}
