//! # MSG API 客户端
//!
//! 为 MSG 消息中心提供声明式 Feign HTTP 客户端，
//! 通过 `#[feign]` 宏自动生成各资源的 CRUD 方法。

mod api_client;

pub use api_client::*;

pub use msg_api::dto;
pub use msg_api::vo;
