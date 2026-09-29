//! # MSG Publisher 配置
//!
//! `ApiClientConfig`（Simple/MicroSvc）来自调用方 AppConfig 的 `api` map，
//! msg-pub 只保管自身的发布策略配置。

use arc_swap::ArcSwapOption;
use robotech::api_client::ApiClientConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

static API_CONFIG: ArcSwapOption<ApiClientConfig> = ArcSwapOption::const_empty();
static PUB_CONFIG: ArcSwapOption<MsgPubConfig> = ArcSwapOption::const_empty();

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct MsgPubConfig {
    /// 发布主题
    #[serde(default = "topic_default")]
    pub topic: String,
}

impl Default for MsgPubConfig {
    fn default() -> Self {
        Self {
            topic: topic_default(),
        }
    }
}

fn topic_default() -> String {
    "msg:message".to_string()
}

pub fn setup(api: ApiClientConfig, config: MsgPubConfig) {
    API_CONFIG.store(Some(Arc::new(api)));
    PUB_CONFIG.store(Some(Arc::new(config)));
}

pub fn get() -> Result<Arc<MsgPubConfig>, Box<dyn std::error::Error + Send + Sync>> {
    PUB_CONFIG
        .load()
        .clone()
        .ok_or_else(|| "MSG Pub 未初始化，请先调用 setup()".into())
}

/// 获取存储的 API 客户端配置，用于创建 feign 客户端调用 msg-svr。
pub fn get_api_config(
) -> Result<Arc<ApiClientConfig>, Box<dyn std::error::Error + Send + Sync>> {
    API_CONFIG
        .load_full()
        .ok_or_else(|| "MSG Pub 未初始化，请先调用 setup()".into())
}