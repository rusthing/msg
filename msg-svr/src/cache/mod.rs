//! # 消息缓存模块
//!
//! 将消息模板、渠道配置、消息队列等低频变更数据从数据库加载到内存，利用
//! [`ArcSwap`] 实现无锁读取与原子热更新。
//!
//! ## 模块结构
//!
//! - [`msg_cache`] — 缓存数据结构定义、消息缓存与队列缓存的双轨管理
//!   - [`MsgCache`] / [`get_msg_cache`] — 消息缓存（模板、渠道、目标等）
//!   - [`MSG_QUEUE_CACHE`] / [`get_queue_cache`] — 队列缓存（仅用于订阅管理）
//!   - [`refresh_msg_message_cache`] — 刷新消息缓存（不触发重新订阅）
//!   - [`refresh_msg_queue_cache`] — 刷新队列缓存（触发重新订阅）

mod msg_cache;

pub use msg_cache::*;