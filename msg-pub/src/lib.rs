//! # MSG Publisher
//!
//! 应用通过 msg-pub 向消息中心发送消息。
//!
//! ## 使用
//!
//! ```ignore
//! use msg_pub::MsgPubConfig;
//!
//! // ApiClientConfig 来自调用方 AppConfig.api map
//! let api_cfg = app_config.api.get("msg-svr").cloned()
//!     .expect("AppConfig 中缺少 msg-svr 的 api 配置");
//!
//! msg_pub::setup(api_cfg, MsgPubConfig::default());
//!
//! msg_pub::publish("order_shipped", params, 1001).await?;
//! ```
//!
//! ### 从配置中心加载 msg-pub 自身配置（可选）
//!
//! ```toml
//! [msg.pub]
//! publish-retry-count = 3
//! publish-retry-interval = "2s"
//! ```
//!
//! ```ignore
//! let pub_cfg: MsgPubConfig = config.get("msg.pub").unwrap_or_default();
//! msg_pub::setup(api_cfg, pub_cfg);
//! ```

pub mod config;
pub mod utils;

pub use config::{get, setup, MsgPubConfig};
pub use utils::*;
