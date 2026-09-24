//! # MSG Publisher 配置
//!
//! 管理 msg-pub 的运行参数，包括 msg-svr 连接地址与重试策略。

use arc_swap::ArcSwapOption;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use wheel_rs::serde::duration_serde;

/// 全局配置缓存（热更新安全）
static MSG_PUB_CONFIG: ArcSwapOption<MsgPubConfig> = ArcSwapOption::const_empty();

/// # Publisher 配置
///
/// 应用程序通过 msg-pub 向消息中心发送消息，本配置指定 msg-svr 地址和重试策略。
///
/// ## 配置示例 (TOML)
///
/// ```toml
/// [msg.pub]
/// msg-svr-base-url = "http://msg-svr:9002"
/// publish-retry-count = 2
/// publish-retry-interval = "1s"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MsgPubConfig {
    /// msg-svr 服务端地址，例如 `"http://msg-svr:9002"`
    ///
    /// 若不配置则需调用 [`setup`] 传入。
    #[serde(default)]
    pub msg_svr_base_url: Option<String>,

    /// 发送消息失败时的最大重试次数，默认 2
    #[serde(default = "publish_retry_count_default")]
    pub publish_retry_count: u32,

    /// 发送消息重试间隔，默认 1s
    #[serde(with = "duration_serde", default = "publish_retry_interval_default")]
    pub publish_retry_interval: Duration,
}

fn publish_retry_count_default() -> u32 {
    2
}

fn publish_retry_interval_default() -> Duration {
    Duration::from_secs(1)
}

/// 直接设置 Publisher 配置（无需配置中心）
///
/// 适用于简单场景，传入 msg-svr 地址即可，其余参数使用默认值。
pub fn setup(msg_svr_base_url: &str) {
    let config = MsgPubConfig {
        msg_svr_base_url: Some(msg_svr_base_url.to_string()),
        publish_retry_count: publish_retry_count_default(),
        publish_retry_interval: publish_retry_interval_default(),
    };
    MSG_PUB_CONFIG.store(Some(Arc::new(config)));
}

/// 从配置中心加载 Publisher 配置
///
/// 适用于使用 robotech 配置中心的应用，从统一的 config tree 中读取。
///
/// ## 参数
///
/// * `config` - robotech 的 Config 对象
/// * `key` - 配置键路径，如 `"msg.pub"`
pub async fn setup_from_cfg(
    config: &config::Config,
    key: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pub_config: MsgPubConfig = config
        .get(key)
        .map_err(|e| format!("加载 MSG Pub 配置失败: {e}"))?;
    MSG_PUB_CONFIG.store(Some(Arc::new(pub_config)));
    Ok(())
}

/// 获取当前配置
///
/// 配置未初始化时返回错误。
pub fn get() -> Result<Arc<MsgPubConfig>, Box<dyn std::error::Error + Send + Sync>> {
    MSG_PUB_CONFIG
        .load()
        .clone()
        .ok_or_else(|| "MSG Pub 配置未初始化，请先调用 setup() 或 setup_from_cfg()".into())
}