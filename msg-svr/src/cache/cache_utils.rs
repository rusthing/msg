//! # 缓存装配
//!
//! 把各份缓存的启动装配收敛在这里：注册刷新信号消费者 + 首次全量加载。
//! `main.rs` 只调用 [`setup_cache`]，不再关心两者之间的先后顺序。
//!
//! 与 dtw-svr 的 `cache/cache_utils.rs` 保持一致：`{对象}_cache.rs` 只放
//! 数据结构、全局静态与 build/store/on_refresh，装配编排统一放本文件。

use crate::cache::{MSG_MESSAGE_CACHE, refresh_message_cache};
use msg_api::cst::MSG_MESSAGE_REFRESH_SUBJECT;
use robotech::cache::setup_local_cache;

/// # 装配消息缓存
///
/// ## 步骤
///
/// 1. 注册 JetStream 刷新信号消费者：任一节点写库后发布信号，各节点全量重建
/// 2. 启动后首次全量加载：消息模板必须先就绪，订阅到消息时才有模板可用
///
/// ## 错误
///
/// 刷新信号订阅失败或首次全量加载失败时返回错误，由 `main.rs` 终止启动。
pub async fn setup_cache() -> anyhow::Result<()> {
    // 消息缓存：收到刷新信号后全量重建缓存快照
    setup_local_cache(&MSG_MESSAGE_REFRESH_SUBJECT, &MSG_MESSAGE_CACHE, || async {
        refresh_message_cache().await
    })
    .await?;

    // 启动后首次全量加载：消息模板必须先就绪，订阅到消息时才有模板可用
    refresh_message_cache().await?;

    Ok(())
}
