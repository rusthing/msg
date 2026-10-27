//! # 消息缓存模块
//!
//! 将消息模板、渠道配置、目标类别等低频变更数据从数据库加载到内存，利用
//! [`ArcSwapOption`] 实现无锁读取与原子热更新，并通过 JetStream 刷新信号
//! 在所有节点间同步缓存变更。
//!
//! 本模块结构与 dtw-svr 的 `cache` 模块一致，遵循 robotech 刷新信号规范。
//!
//! ## 模块结构
//!
//! - [`message_cache`] — 消息缓存数据结构定义与消息缓存管理
//!   - [`get_message_cache`] — 返回 `Arc<HashMap<String, MessageCache>>` 消息缓存快照
//!   - [`MSG_MESSAGE_CACHE`] — `pub static` 全局缓存，供 [`setup_cache`] 注册刷新信号订阅
//!   - [`refresh_message_cache`] — 全量刷新（DB + build + store）
//!   - [`store_message_cache`] — 原子替换缓存快照
//!   - [`on_message_cache_refresh`] — 写操作后发布消息缓存刷新信号
//! - [`cache_utils`] — 启动装配
//!   - [`setup_cache`] — 注册刷新信号消费者 + 首次全量加载，由 `main.rs` 调用
//!
//! ## 分层
//!
//! | 文件 | 职责 |
//! |------|------|
//! | `{对象}_cache.rs` | 数据结构、全局静态、build / store / on_refresh / refresh |
//! | `cache_utils.rs` | 启动装配（`setup_cache`），不持有任何缓存状态 |
//!
//! ## 命名约定
//!
//! 文件、静态变量、读写函数统一以**缓存对象**命名，而不是笼统的
//! `xxx_cache` / `CACHE`：
//!
//! | 层次 | 命名 | 示例 |
//! |------|------|------|
//! | 模块文件 | `{对象}_cache.rs` | `message_cache.rs` |
//! | 全局静态 | `{服务}_{对象}_CACHE` | `MSG_MESSAGE_CACHE` |
//! | 读 / 写 / 构建 | `get_/store_/build_{对象}_cache` | `get_message_cache` |
//! | 全量刷新 | `refresh_{对象}_cache` | `refresh_message_cache` |
//! | 写后回调 | `on_{对象}_cache_refresh` | `on_message_cache_refresh` |
//! | 刷新主题 | `refresh.{服务}.{对象}` | `refresh.msg.message` |
//!
//! 新增第二份缓存时（如模板缓存）只需按上表补一套
//! `template_cache.rs` / `MSG_TEMPLATE_CACHE` / `refresh.msg.template`，
//! 不与已有缓存抢名字。
//!
//! ## 同步机制
//!
//! 任一节点写库成功后发布刷新信号（JetStream 持久化），所有节点收到信号后
//! 从 DB 全量重建缓存，无需配置中心参与。
//!
//! ## 队列不在其中
//!
//! 消息队列（`msg_message_queue`）只用于决定订阅哪些 subject，订阅状态由
//! `crate::sub` 维护，没有读取方需要队列快照，因此不进缓存。队列变更直接驱动
//! 订阅同步，见 [`crate::sub::reload_message_queue_subscriptions`]；启动装配在
//! [`crate::utils::setup_message_queue_refresh`]，与本模块的 [`setup_cache`] 平级。
//!
//! 投递队列（`msg_delivery_queue`）不在这里：发布侧每次投递都要查「这个渠道
//! 发到哪个主题」，快照得常驻内存，因此和投递订阅状态一起放在
//! [`crate::wkr::sub`] 里维护。

mod cache_utils;
mod message_cache;

pub use cache_utils::*;
pub use message_cache::*;
