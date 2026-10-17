//! # 消息缓存模块
//!
//! 将消息模板、渠道配置、消息队列等低频变更数据从数据库加载到内存，利用
//! [`ArcSwapOption`] 实现无锁读取与原子热更新，并通过 JetStream 刷新信号
//! 在所有节点间同步缓存变更。
//!
//! 本模块结构与 [`dtw_rs::cache::dtw_local_cache`] 一致，遵循 robotech 本地缓存规范。
//!
//! ## 模块结构
//!
//! - [`msg_local_cache`] — 缓存数据结构定义、消息缓存与队列缓存的双轨管理
//!   - [`get_msg_cache`] — 返回 `Arc<HashMap<String, MessageCache>>` 消息缓存快照
//!   - [`MSG_CACHE`] / [`MSG_QUEUE_CACHE`] — `pub static` 全局缓存，供 `main.rs` 注册 JetStream 消费者
//!   - [`refresh_msg_cache`] / [`refresh_queue_cache`] — 全量刷新（DB + build + store）
//!   - [`store_msg_cache`] / [`store_queue_cache`] — 原子替换缓存快照
//!   - [`on_cache_refresh`] — 写操作后发布消息缓存刷新信号
//!   - [`on_queue_cache_refresh`] — 写操作后发布队列缓存刷新信号
//!
//! ## 同步机制
//!
//! 任一节点写库成功后发布刷新信号（JetStream 持久化），所有节点收到信号后
//! 从 DB 全量重建缓存，无需配置中心参与。

mod msg_local_cache;

pub use msg_local_cache::*;