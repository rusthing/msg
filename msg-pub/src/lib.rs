//! # MSG Publisher
//!
//! 应用通过 msg-pub 向消息中心发送消息。
//!
//! ## 使用
//!
//! ```ignore
//! use msg_pub;
//! use std::collections::HashMap;
//!
//! // 初始化
//! msg_pub::setup("http://msg-svr:9002");
//!
//! // 发送
//! let mut params = HashMap::new();
//! params.insert("order_id".to_string(), "ORD-20240001".to_string());
//! let delivery_id = msg_pub::publish("order_shipped", params, 1001).await?;
//! ```

pub mod config;
pub mod utils;

pub use config::MsgPubConfig;
pub use config::msg_pub_config::{get, setup, setup_from_cfg};
pub use utils::{
    publish, publish_with_retry, PublishRequest, PublishResponse,
};