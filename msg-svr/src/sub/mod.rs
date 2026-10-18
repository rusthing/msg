//! # 消息队列订阅管理
//!
//! 根据数据库中的消息队列配置启动或停止 NATS 订阅线程。
//! 订阅状态由本模块自己维护（不经过缓存），队列变更通过刷新信号通知：
//!
//! ```text
//! 队列写操作 → on_queue_refresh() 发布信号
//!   → 各节点 reload_queue_subscriptions()
//!     → 查库 → sync_queue_subscriptions() → 增删订阅
//! ```
//!
//! 使用 `arc_swap` 持有的句柄确保线程安全，支持热更新。

mod dead_letter;
mod message_sub;

pub use dead_letter::*;
pub use message_sub::*;
