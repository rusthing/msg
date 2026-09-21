//! # MSG Worker Client Library
//!
//! 为消息 Worker 提供的简化工具函数，封装了与 MSG 服务端的 HTTP 通信，
//! 提供拉取待处理消息、上报处理结果等常用操作。

pub mod config;
pub mod utils;

pub use config::MsgWorkerConfig;
pub use utils::{setup_msg_worker, MsgMessageHandler};