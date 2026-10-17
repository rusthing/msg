use crate::config::MsgConfig;
use idworker::IdWorkerConfig;
use robotech::db::DbConnConfig;
use robotech::mq::nats::NatsConfig;
use robotech::redis::RedisConfig;
use robotech::web::WebServerConfig;
use serde::Deserialize;

/// 配置文件结构
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub struct AppConfig {
    /// db
    pub db: DbConnConfig,
    /// msg
    #[serde(default = "MsgConfig::default")]
    pub msg: MsgConfig,
    /// Web服务器
    #[serde(default = "WebServerConfig::default")]
    pub web: WebServerConfig,
    /// id_worker
    #[serde(default = "IdWorkerConfig::default")]
    pub id_worker: IdWorkerConfig,
    /// redis
    pub redis: Option<RedisConfig>,
    /// nats
    pub nats: Option<NatsConfig>,
}