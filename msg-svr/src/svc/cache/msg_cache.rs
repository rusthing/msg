//! # 消息缓存
//!
//! 将消息模板、渠道、队列、目标类别等低频变更数据从数据库加载到内存，
//! 利用 [`ArcSwap`] 实现无锁并发读取与原子热更新。
//!
//! ## 数据结构
//!
//! - [`MsgCache`] — 缓存快照，包含消息模板及其关联的所有配置数据
//! - [`CachedMessage`] — 单条消息的完整缓存条目
//! - [`CachedChannel`] — 渠道配置缓存
//!
//! ## 使用
//!
//! ```ignore
//! let cache = get_msg_cache();
//! if let Some(msg) = cache.messages.get("order.created") {
//!     for ch in &msg.channels {
//!         let channel = &cache.channels[ch];
//!         // 通过 channel 发送消息...
//!     }
//! }
//! ```

use arc_swap::ArcSwapOption;
use config::Value;
use robotech::db::get_db_conn;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use wheel_rs::config_utils::has_config_changed;

// ── 实体引用 ──
use msg_api::mo::prelude::*;
use msg_api::mo::{msg_channel, msg_message};

/// # 消息缓存键前缀
///
/// 用于在 `changed` Map 中判断是否需要刷新缓存；
/// 缓存数据来源于数据库，此处绑定到 `"db"` 键，DB 重连时自动刷新。
const MSG_CACHE_KEY_PREFIX: &str = "db";

/// # 全局消息缓存
///
/// 使用 [`ArcSwapOption`] 存储，无锁读取；`const_empty` 保证启动时可用。
static MSG_CACHE: ArcSwapOption<MsgCache> = ArcSwapOption::const_empty();

// ═══════════════════════════════════════════════════════════════
//  数据结构
// ═══════════════════════════════════════════════════════════════

/// # 消息缓存快照
///
/// 包含系统中全部启用状态的配置数据：
/// - [`messages`] — 以 `msg_message.code` 为键，聚合了该消息的队列、类别、来源、渠道、目标
/// - [`channels`] — 以 `msg_channel.code` 为键的渠道配置
/// - [`sources`] — 以 `msg_message_source.code` 为键的消息来源
/// - [`categories`] — 以 `msg_message_category.code` 为键的消息类别
/// - [`target_categories`] — 以 `msg_target_category.code` 为键的目标类别
#[derive(Debug, Clone, Default)]
pub struct MsgCache {
    /// 消息模板，key = `msg_message.code`
    pub messages: HashMap<String, CachedMessage>,
    /// 渠道配置，key = `msg_channel.code`
    pub channels: HashMap<String, CachedChannel>,
    /// 消息来源，key = `msg_message_source.code`
    pub sources: HashMap<String, CachedSource>,
    /// 消息类别，key = `msg_message_category.code`
    pub categories: HashMap<String, CachedCategory>,
    /// 目标类别，key = `msg_target_category.code`
    pub target_categories: HashMap<String, CachedCategory>,
}

/// # 缓存的单条消息
///
/// 聚合了消息模板及其关联的队列、类别、来源、渠道列表、目标列表。
#[derive(Debug, Clone)]
pub struct CachedMessage {
    /// 消息 ID
    pub id: i64,
    /// 编码，唯一标识
    pub code: String,
    /// 名称
    pub name: String,
    /// 标题模板
    pub title_template: String,
    /// 内容模板
    pub content_template: String,
    /// 备注
    pub remark: Option<String>,
    /// 关联的消息队列
    pub queue: CachedQueue,
    /// 关联的消息类别（`category_id` 可能为空）
    pub category: Option<CachedCategory>,
    /// 关联的消息来源（`source_id` 可能为空）
    pub source: Option<CachedSource>,
    /// 该消息绑定的渠道 code 列表
    pub channel_codes: Vec<String>,
    /// 该消息的目标配置列表
    pub targets: Vec<CachedTarget>,
}

/// # 缓存的消息队列
#[derive(Debug, Clone)]
pub struct CachedQueue {
    /// 队列 ID
    pub id: i64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化（`true` → JetStream，`false` → Core NATS）
    pub persisted: bool,
}

/// # 缓存的消息来源
#[derive(Debug, Clone)]
pub struct CachedSource {
    /// 来源 ID
    pub id: i64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
}

/// # 缓存的消息类别（同时用于目标类别）
#[derive(Debug, Clone)]
pub struct CachedCategory {
    /// 类别 ID
    pub id: i64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
}

/// # 缓存的渠道配置
#[derive(Debug, Clone)]
pub struct CachedChannel {
    /// 渠道 ID
    pub id: i64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 渠道选项（JSON 字符串）
    pub options: Option<String>,
    /// 署名
    pub remark: Option<String>,
}

/// # 缓存的消息目标
#[derive(Debug, Clone)]
pub struct CachedTarget {
    /// 目标类别 code
    pub target_category_code: String,
    /// 目标 ID（`None` 表示该类别下的所有目标）
    pub target_id: Option<i64>,
}

// ═══════════════════════════════════════════════════════════════
//  公开 API
// ═══════════════════════════════════════════════════════════════

/// # 获取全局消息缓存快照
///
/// 返回当前缓存快照的只读引用，调用方仅持有 [`Arc`] 共享所有权，无需拷贝。
/// 若缓存尚未初始化，返回空的默认缓存。
///
/// ## 返回值
///
/// [`Arc<MsgCache>`] — 当下的缓存快照
pub fn get_msg_cache() -> Arc<MsgCache> {
    MSG_CACHE
        .load_full()
        .unwrap_or_else(|| Arc::new(MsgCache::default()))
}

/// # 初始化或热更新消息缓存
///
/// 通常由 `bootstrap!` 的 `setup()` 回调调用。当 `changed` 为 `None`
/// （首次加载）或 `"db"` 配置变更（DB 重连）时，全量从 DB 刷新缓存。
///
/// ## 参数
///
/// - `changed`: 配置变更集合，`None` 表示首次加载
pub async fn setup_msg_cache(changed: &Option<HashMap<String, Value>>) {
    if changed
        .as_ref()
        .map(|c| has_config_changed(MSG_CACHE_KEY_PREFIX, c))
        .unwrap_or(true)
    {
        info!("刷新消息缓存...");
        match refresh_msg_cache().await {
            Ok(()) => info!("消息缓存刷新完成"),
            Err(e) => error!("刷新消息缓存失败: {e}"),
        }
    }
}

/// # 全量刷新消息缓存
///
/// 从数据库加载消息、渠道、队列、来源、类别、目标类别、消息-渠道关联、
/// 消息-目标关联，组装为 [`MsgCache`] 并通过 [`ArcSwap`] 原子替换。
///
/// ## 返回值
///
/// 刷新成功返回 `Ok(())`；数据库错误返回 `Err`
pub async fn refresh_msg_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = get_db_conn()?;

    // ── 加载各类基础数据 ──

    let channels: HashMap<String, CachedChannel> = MsgChannel::find()
        .filter(msg_channel::Column::Enabled.eq(true))
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| {
            (
                m.code.clone(),
                CachedChannel {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                    options: m.options,
                    remark: m.remark,
                },
            )
        })
        .collect();

    let sources: HashMap<String, CachedSource> = MsgMessageSource::find()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| {
            (
                m.code.clone(),
                CachedSource {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                },
            )
        })
        .collect();

    let categories: HashMap<String, CachedCategory> = MsgMessageCategory::find()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| {
            (
                m.code.clone(),
                CachedCategory {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                },
            )
        })
        .collect();

    let target_categories: HashMap<String, CachedCategory> = MsgTargetCategory::find()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| {
            (
                m.code.clone(),
                CachedCategory {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                },
            )
        })
        .collect();

    let queues: HashMap<i64, CachedQueue> = MsgMessageQueue::find()
        .all(db.as_ref())
        .await?
        .into_iter()
        .map(|m| {
            (
                m.id,
                CachedQueue {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                    persisted: m.persisted,
                },
            )
        })
        .collect();

    // ── 消息-渠道关联 (message_id → Vec<channel_code>) ──

    let message_channels: HashMap<i64, Vec<String>> = {
        let links = MsgMessageChannel::find().all(db.as_ref()).await?;
        let channel_map: HashMap<i64, String> = MsgChannel::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|c| (c.id, c.code))
            .collect();

        let mut map: HashMap<i64, Vec<String>> = HashMap::new();
        for link in links {
            if let Some(ch_code) = channel_map.get(&link.channel_id) {
                map.entry(link.message_id).or_default().push(ch_code.clone());
            }
        }
        map
    };

    // ── 消息-目标关联 (message_id → Vec<CachedTarget>) ──

    let message_targets: HashMap<i64, Vec<CachedTarget>> = {
        let links = MsgMessageTarget::find().all(db.as_ref()).await?;
        let tc_map: HashMap<i64, String> = MsgTargetCategory::find()
            .all(db.as_ref())
            .await?
            .into_iter()
            .map(|t| (t.id, t.code))
            .collect();

        let mut map: HashMap<i64, Vec<CachedTarget>> = HashMap::new();
        for link in links {
            if let Some(tc_code) = tc_map.get(&link.target_category_id) {
                map.entry(link.message_id).or_default().push(CachedTarget {
                    target_category_code: tc_code.clone(),
                    target_id: link.target_id,
                });
            }
        }
        map
    };

    // ── 组装消息 ──

    let messages: HashMap<String, CachedMessage> = MsgMessage::find()
        .filter(msg_message::Column::Enabled.eq(true))
        .all(db.as_ref())
        .await?
        .into_iter()
        .filter_map(|m| {
            let queue = queues.get(&m.mes_id)?.clone();
            let category = m.category_id.and_then(|cid| {
                categories.values().find(|v| v.id == cid).map(|v| v.clone())
            });
            let source = m.source_id.and_then(|sid| {
                sources.values().find(|v| v.id == sid).map(|v| v.clone())
            });
            let channel_codes = message_channels.get(&m.id).cloned().unwrap_or_default();
            let targets = message_targets.get(&m.id).cloned().unwrap_or_default();

            Some((
                m.code.clone(),
                CachedMessage {
                    id: m.id,
                    code: m.code,
                    name: m.name,
                    title_template: m.title_template,
                    content_template: m.content_template,
                    remark: m.remark,
                    queue,
                    category,
                    source,
                    channel_codes,
                    targets,
                },
            ))
        })
        .collect();

    // ── 原子替换 ──

    let cache = MsgCache {
        messages,
        channels,
        sources,
        categories,
        target_categories,
    };

    MSG_CACHE.store(Some(Arc::new(cache)));

    Ok(())
}