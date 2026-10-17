//! # MSG Publisher 配置
//!
//! `ApiClientConfig`（Simple/MicroSvc）来自调用方 AppConfig 的 `api` map，
//! msg-pub 只保管自身的发布策略配置。

use derive_setters::Setters;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(o2o, Debug, Clone, Serialize, Deserialize, Setters, TypedBuilder)]
#[owned_into(msg_api_client::dto::MsgEventSourceAddDto)]
#[ghosts(
    id: Default::default(),
    _current_ms: Default::default(),
    _current_user_id: robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID.into(),
)]
#[serde(rename_all = "kebab-case")]
pub struct MsgEventSourceConfig {
    #[into(Some(~))]
    pub code: String,
    #[into(Some(~))]
    pub name: String,
    #[into(Some(~))]
    #[serde(default)]
    #[builder(default)]
    pub remark: Option<String>,
}
