//! # MSG 缓存 NATS 主题常量
//!
//! 本地缓存的跨节点同步通过 JetStream 刷新信号完成：
//! 任一节点写库后发布信号，所有节点收到信号后从 DB 全量重建缓存。

/// 消息缓存全量刷新 JetStream 主题
///
/// 消息模板、事件来源、类别、目标类别、渠道等数据变更时发布。
pub const MSG_CACHE_REFRESH_SUBJECT: &str = "msg.cache.refresh";

/// 消息队列缓存全量刷新 JetStream 主题
///
/// 消息队列变更时发布，刷新队列缓存并同步 NATS 订阅。
pub const MSG_QUEUE_CACHE_REFRESH_SUBJECT: &str = "msg.queue.cache.refresh";
