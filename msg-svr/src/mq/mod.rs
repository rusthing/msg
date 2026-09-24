//! # 消息队列订阅管理
//!
//! 根据缓存中的消息队列信息启动或停止 NATS 订阅线程。
//! 使用 `arc_swap` 持有的句柄确保线程安全，支持热更新。

mod message_handler;
mod queue_subscriber;

pub use message_handler::process_message;
pub use queue_subscriber::*;