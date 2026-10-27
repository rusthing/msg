//! # MSG Publisher
//!
//! 应用通过 msg-pub 向消息中心发送消息：`MessageMsg` 报文由 msg-svr 按模板
//! 渲染后再投递到各渠道。
//!
//! ## 用法
//!
//! ```ignore
//! use msg_pub::msg::message_msg::MessageMsg;
//! use msg_pub::{PublishMode, publish_message, publish_message_to};
//!
//! // 发到默认队列（`msg.message.normal`），走 JetStream 并等待 ACK
//! publish_message(message_msg).await?;
//!
//! // 发到自己注册的队列，按业务对可靠性的要求选模式
//! publish_message_to("msg.message.telemetry", message_msg, PublishMode::BestEffort).await?;
//! ```
//!
//! ## 自己的消息队列
//!
//! 默认队列之外的 subject 必须先注册到消息中心（`msg_message_queue` 表），
//! 消息中心只会订阅登记过的队列——没登记的 subject 发出去没人收：
//!
//! ```ignore
//! let queue = MsgMessageQueueConfig::builder()
//!     .code("msg.message.telemetry".to_string())
//!     .name("遥测消息队列".to_string())
//!     .persisted(false)
//!     .build();
//! api_client.message_queue.add(&queue.into()).await?;
//! ```

mod config;
mod utils;

pub use config::*;
pub use utils::*;

pub use msg_api_client::*;
