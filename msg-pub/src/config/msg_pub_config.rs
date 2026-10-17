//! # MSG Publisher 配置
//!
//! `ApiClientConfig`（Simple/MicroSvc）来自调用方 AppConfig 的 `api` map，
//! msg-pub 只保管自身的发布策略配置。

use serde::{Deserialize, Serialize};

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
