use arc_swap::ArcSwapOption;
use robotech::cfg::CfgError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use wheel_rs::serde::duration_serde;

pub const MSG_WORKER_CONFIG_KEY: &str = "msg.worker";
static MSG_WORKER_CONFIG: ArcSwapOption<MsgWorkerConfig> = ArcSwapOption::const_empty();

pub fn get_msg_worker_config() -> Result<Arc<MsgWorkerConfig>, CfgError> {
    MSG_WORKER_CONFIG.load_full().ok_or(CfgError::NotInit(
        "Worker config not initialized".to_string(),
    ))
}

pub fn set_msg_worker_config(config: MsgWorkerConfig) {
    MSG_WORKER_CONFIG.store(Some(Arc::new(config.clone())));
}

/// Worker 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MsgWorkerConfig {
    /// Redis Stream 的 key前缀
    #[serde(default = "worker_key_default")]
    pub worker_key: String,
    /// Worker 编码
    pub worker_code: String,
    /// Worker 订阅消息分组
    #[serde(default = "worker_group_default")]
    pub worker_group: Option<String>,
    /// 扫描间隔
    #[serde(with = "duration_serde", default = "scan_interval_default")]
    pub scan_interval: Duration,
    /// 扫描阻塞时间
    #[serde(with = "duration_serde", default = "scan_block_default")]
    pub scan_block_duration: Duration,
    /// 单次接收消息的最大数量
    #[serde(default = "max_messages_default")]
    pub max_messages: usize,
    /// 上报结果失败时最大重试次数
    #[serde(default = "report_retry_count_default")]
    pub report_retry_count: u32,
    /// 上报结果重试间隔
    #[serde(with = "duration_serde", default = "report_retry_interval_default")]
    pub report_retry_interval: Duration,
}

fn worker_key_default() -> String {
    "msg:stream:".to_string()
}

fn worker_group_default() -> Option<String> {
    None
}

fn scan_interval_default() -> Duration {
    Duration::from_secs(5)
}

fn scan_block_default() -> Duration {
    Duration::from_secs(2)
}

fn max_messages_default() -> usize {
    10
}

fn report_retry_count_default() -> u32 {
    3
}

fn report_retry_interval_default() -> Duration {
    Duration::from_secs(1)
}