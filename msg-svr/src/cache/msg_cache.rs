//! # 消息缓存
//!
//! 将消息模板、渠道、队列、目标类别等低频变更数据从数据库加载到内存，
//! 利用 [`ArcSwap`] 实现无锁并发读取与原子热更新。
//!
//! ## 数据结构
//!
//! - [`MsgCache`] — 缓存快照，包含消息模板及其关联的所有配置数据
//! - [`MessageCache`] — 单条消息的完整缓存条目
//! - [`ChannelCache`] — 渠道配置缓存
//!
//! ## 使用
//!
//! ```ignore
//! let cache = get_msg_cache();
//! if let Some(msg) = cache.messages.get("order.created") {
//!     for ch in &msg.channels {
//!         // 直接用 ch.options / ch.remark 发送消息
//!     }
//! }
//! ```

use arc_swap::ArcSwapOption;
use config::Value;
use robotech::db::{get_db_conn, DB_CONN_CONFIG_KEY};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{error, info};
use wheel_rs::config_utils::has_config_changed;

// ── SVC 引用 ──
use crate::dto::{
    MsgChannelQueryDto, MsgEventSourceQueryDto, MsgMessageCategoryQueryDto, MsgMessageQueryDto,
    MsgMessageQueueQueryDto, MsgMessageTargetQueryDto, MsgTargetCategoryChannelQueryDto,
    MsgTargetCategoryQueryDto,
};
use crate::sub::sync_queue_subscriptions;
use crate::svc::{
    MsgChannelSvc, MsgEventSourceSvc, MsgMessageCategorySvc, MsgMessageQueueSvc, MsgMessageSvc,
    MsgMessageTargetSvc, MsgTargetCategoryChannelSvc, MsgTargetCategorySvc,
};
use crate::vo::{
    MsgChannelVo, MsgMessageExVo, MsgMessageQueueVo, MsgMessageTargetVo, MsgTargetCategoryChannelVo,
};

/// # 消息缓存配置中心版本键
///
/// 配置中心中 `refresh-scope.msg-message` 的值变化时，
/// 说明消息数据有更新，需要刷新消息缓存（不会触发队列重新订阅）。
const MSG_MESSAGE_CACHE_CONFIG_KEY: &str = "refresh-scope.msg-message";

/// # 消息队列缓存配置中心版本键
///
/// 配置中心中 `refresh-scope.msg-message-queue` 的值变化时，
/// 说明消息队列数据有更新，需要刷新队列缓存并触发重新订阅。
const MSG_MESSAGE_QUEUE_CACHE_CONFIG_KEY: &str = "refresh-scope.msg-message-queue";

/// # 全局消息缓存
///
/// 使用 [`ArcSwapOption`] 存储，无锁读取；`const_empty` 保证启动时可用。
static MSG_CACHE: ArcSwapOption<MsgCache> = ArcSwapOption::const_empty();

/// # 全局消息队列缓存
///
/// 独立于消息缓存的队列缓存，用于订阅管理。
/// key = `queue.code`，value = [`QueueCache`]。
/// 使用 [`ArcSwapOption`] 存储，无锁读取。
static MSG_QUEUE_CACHE: ArcSwapOption<HashMap<String, QueueCache>> = ArcSwapOption::const_empty();

// ═══════════════════════════════════════════════════════════════
//  数据结构
// ═══════════════════════════════════════════════════════════════

/// # 消息缓存快照
///
/// 刷新时分表查询、组装时全量内嵌，运行时只需一个 `messages` 字典。
/// 目标类别数据已内嵌在各消息的 `TargetCache.target_category` 中，无需单独缓存。
#[derive(Debug, Clone, Default)]
pub struct MsgCache {
    /// 消息模板，key = `msg_message.event_code`
    pub messages: HashMap<String, MessageCache>,
}

/// # 缓存的单条消息
///
/// 聚合了消息模板及其关联的队列、类别、来源、渠道列表、目标列表。
#[derive(Debug, Clone)]
pub struct MessageCache {
    /// 消息 ID
    pub id: u64,
    /// 事件编码，唯一标识
    pub event_code: String,
    /// 名称
    pub name: String,
    /// 是否持久化（如果不持久化，投递将不会保存到数据库）
    pub persisted: bool,
    /// 标题模板（为 null 的话从 annotations 中取 title 设置为标题）
    pub title_template: Option<String>,
    /// 内容模板（为 null 的话从 annotations 中取 content 设置为内容）
    pub content_template: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 关联的消息队列
    pub queue: QueueCache,
    /// 关联的消息类别
    pub category: CategoryCache,
    /// 关联的消息事件来源
    pub source: SourceCache,
    /// 该消息绑定的渠道列表（含完整配置，刷新时组装）
    pub channels: Vec<ChannelCache>,
    /// 该消息的目标配置列表
    pub targets: Vec<TargetCache>,
}

/// # 缓存的消息队列
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueueCache {
    /// 队列 ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否持久化（`true` → JetStream，`false` → Core NATS）
    pub persisted: bool,
}

/// # 缓存的消息来源
#[derive(Debug, Clone)]
pub struct SourceCache {
    /// 来源 ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
}

/// # 缓存的消息类别（同时用于目标类别）
#[derive(Debug, Clone)]
pub struct CategoryCache {
    /// 类别 ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
}

/// # 缓存的渠道配置
#[derive(Debug, Clone)]
pub struct ChannelCache {
    /// 渠道 ID
    pub id: u64,
    /// 渠道编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 渠道选项（JSON 字符串）
    pub options: Option<String>,
    /// 署名
    pub remark: Option<String>,
}

/// # 缓存的目标类别
///
/// 聚合了目标类别基本信息及其渠道列表，处理消息投递时可直接拿到该目标类别
/// 关联的渠道，无需额外查询。
#[derive(Debug, Clone)]
pub struct TargetCategoryCache {
    /// 类别 ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 该目标类别绑定的渠道列表（含完整配置，刷新时组装）
    pub channels: Vec<ChannelCache>,
}

/// # 缓存的消息目标
///
/// 直接内嵌 [`TargetCategoryCache`]（含渠道列表），消息投递处理时无需
/// 再通过 `target_category_id` 去额外查找，直接从 `target.target_category.channels`
/// 拿渠道。
#[derive(Debug, Clone)]
pub struct TargetCache {
    pub target_category: TargetCategoryCache,
    pub target_id: u64,
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

/// # 获取全局消息队列缓存快照
///
/// 返回当前队列缓存的只读引用。若缓存尚未初始化，返回空的默认 map。
///
/// ## 返回值
///
/// [`Arc<HashMap<String, QueueCache>>`] — 当下的队列缓存快照
pub fn get_queue_cache() -> Arc<HashMap<String, QueueCache>> {
    MSG_QUEUE_CACHE
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()))
}

/// # 初始化或热更新消息缓存
///
/// 根据配置变更情况，分别刷新消息队列缓存和消息缓存：
/// - `msg-message-queue` 变更 → 刷新队列缓存并触发队列重新订阅
/// - `msg-message` 变更 → 仅刷新消息缓存，不触发重新订阅
/// - `db` 配置变更（DB 重连）→ 两者均刷新
///
/// ## 参数
///
/// - `changed`: 配置变更集合，`None` 表示首次加载
pub async fn setup_msg_cache(changed: &Option<HashMap<String, Value>>) {
    let db_changed = changed
        .as_ref()
        .map(|c| has_config_changed(DB_CONN_CONFIG_KEY, c))
        .unwrap_or(true);

    let queue_changed = db_changed
        || changed
            .as_ref()
            .map(|c| has_config_changed(MSG_MESSAGE_QUEUE_CACHE_CONFIG_KEY, c))
            .unwrap_or(false);

    let message_changed = db_changed
        || changed
            .as_ref()
            .map(|c| has_config_changed(MSG_MESSAGE_CACHE_CONFIG_KEY, c))
            .unwrap_or(false);

    // ── 刷新消息队列缓存（会触发队列重新订阅） ──

    if queue_changed {
        info!("刷新消息队列缓存...");
        match refresh_msg_queue_cache().await {
            Ok(()) => info!("消息队列缓存刷新完成"),
            Err(e) => error!("刷新消息队列缓存失败: {e}"),
        }
    }

    // ── 刷新消息缓存（不会触发队列重新订阅） ──

    if message_changed {
        info!("刷新消息缓存...");
        match refresh_msg_message_cache().await {
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
/// 注意：本函数不会触发队列重新订阅。如需重新订阅，请调用
/// [`refresh_msg_queue_cache`]。
///
/// ## 返回值
///
/// 刷新成功返回 `Ok(())`；数据库错误返回 `Err`
pub async fn refresh_msg_message_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = get_db_conn()?;

    // ── 通过 SVC 分表查询，禁用数据不查出 ──

    let channels_all: Vec<MsgChannelVo> = MsgChannelSvc::list_by_query_dto(
        MsgChannelQueryDto {
            enabled: Some(true),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();
    let channels_by_id: HashMap<u64, &MsgChannelVo> =
        channels_all.iter().map(|c| (c.id.into(), c)).collect();

    let sources: HashMap<u64, SourceCache> =
        MsgEventSourceSvc::list_by_query_dto(MsgEventSourceQueryDto::default(), Some(db.as_ref()))
            .await?
            .extra
            .unwrap_or_default()
            .into_iter()
            .map(|m| {
                (
                    m.id.0,
                    SourceCache {
                        id: m.id.0,
                        code: m.code,
                        name: m.name,
                    },
                )
            })
            .collect();

    let categories: HashMap<u64, CategoryCache> = MsgMessageCategorySvc::list_by_query_dto(
        MsgMessageCategoryQueryDto::default(),
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default()
    .into_iter()
    .map(|m| {
        (
            m.id.0,
            CategoryCache {
                id: m.id.0,
                code: m.code,
                name: m.name,
            },
        )
    })
    .collect();

    // ── 目标类别（含渠道列表），先查出类别、再查出类别-渠道关联、再组装 ──

    let target_category_channels: HashMap<u64, Vec<u64>> = {
        let links: Vec<MsgTargetCategoryChannelVo> =
            MsgTargetCategoryChannelSvc::list_by_query_dto(
                MsgTargetCategoryChannelQueryDto::default(),
                Some(db.as_ref()),
            )
            .await?
            .extra
            .unwrap_or_default();
        let mut map: HashMap<u64, Vec<u64>> = HashMap::new();
        for link in links {
            map.entry(link.target_category_id.0)
                .or_default()
                .push(link.channel_id.0);
        }
        map
    };

    let target_categories: HashMap<u64, TargetCategoryCache> =
        MsgTargetCategorySvc::list_by_query_dto(
            MsgTargetCategoryQueryDto::default(),
            Some(db.as_ref()),
        )
        .await?
        .extra
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let channels: Vec<ChannelCache> = target_category_channels
                .get(&m.id.0)
                .map(|ch_ids| {
                    ch_ids
                        .iter()
                        .filter_map(|ch_id| channels_by_id.get(ch_id))
                        .map(|ch| ChannelCache {
                            id: ch.id.0,
                            code: ch.code.clone(),
                            name: ch.name.clone(),
                            options: ch.options.clone(),
                            remark: ch.remark.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();

            (
                m.id.0,
                TargetCategoryCache {
                    id: m.id.0,
                    code: m.code,
                    name: m.name,
                    channels,
                },
            )
        })
        .collect();

    // ── 消息 + 队列（通过 DAO related_tables + SVC list_ex 一次 join 查询） ──

    let messages_all: Vec<MsgMessageExVo> = MsgMessageSvc::list_ex_by_query_dto(
        MsgMessageQueryDto {
            enabled: Some(true),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();

    // ── 消息-渠道关联（通过消息目标 → 已缓存的目标类别渠道推导） ──

    let message_channels: HashMap<u64, Vec<ChannelCache>> = {
        let links: Vec<MsgMessageTargetVo> = MsgMessageTargetSvc::list_by_query_dto(
            MsgMessageTargetQueryDto::default(),
            Some(db.as_ref()),
        )
        .await?
        .extra
        .unwrap_or_default();
        let mut map: HashMap<u64, Vec<ChannelCache>> = HashMap::new();
        for link in links {
            if let Some(tc) = target_categories.get(&link.target_category_id.0) {
                for ch in &tc.channels {
                    let entry = map.entry(link.message_id.0).or_default();
                    if !entry.iter().any(|c| c.id == ch.id) {
                        entry.push(ch.clone());
                    }
                }
            }
        }
        map
    };

    // ── 消息-目标关联（复用 target_categories） ──

    let message_targets: HashMap<u64, Vec<TargetCache>> = {
        let links: Vec<MsgMessageTargetVo> = MsgMessageTargetSvc::list_by_query_dto(
            MsgMessageTargetQueryDto::default(),
            Some(db.as_ref()),
        )
        .await?
        .extra
        .unwrap_or_default();
        let mut map: HashMap<u64, Vec<TargetCache>> = HashMap::new();
        for link in links {
            if let Some(tc) = target_categories.get(&link.target_category_id.0) {
                map.entry(link.message_id.0).or_default().push(TargetCache {
                    target_category: tc.clone(),
                    target_id: link.target_id.0,
                });
            }
        }
        map
    };

    // ── 组装消息 ──

    let messages: HashMap<String, MessageCache> = messages_all
        .into_iter()
        .map(|m| {
            let cached_queue = QueueCache {
                id: m.msg_message_queue.id.0,
                code: m.msg_message_queue.code.clone(),
                name: m.msg_message_queue.name.clone(),
                persisted: m.msg_message_queue.persisted,
            };
            let category = categories
                .get(&m.category_id.0)
                .cloned()
                .expect("消息类别应存在：category_id 为 NOT NULL FK");
            let source = sources
                .get(&m.event_source_id.0)
                .cloned()
                .expect("消息事件来源应存在：event_source_id 为 NOT NULL FK");
            let channels = message_channels.get(&m.id.0).cloned().unwrap_or_default();
            let targets = message_targets.get(&m.id.0).cloned().unwrap_or_default();

            (
                m.event_code.clone(),
                MessageCache {
                    id: m.id.0,
                    event_code: m.event_code,
                    name: m.name,
                    persisted: m.persisted,
                    title_template: m.title_template,
                    content_template: m.content_template,
                    remark: m.remark,
                    queue: cached_queue,
                    category,
                    source,
                    channels,
                    targets,
                },
            )
        })
        .collect();

    // ── 原子替换 ──

    let cache = MsgCache { messages };

    MSG_CACHE.store(Some(Arc::new(cache)));

    Ok(())
}

/// # 全量刷新消息队列缓存
///
/// 从数据库加载所有消息队列，更新队列缓存，并触发队列重新订阅。
/// 若队列有新增或移除，将自动同步 NATS 订阅状态。
///
/// ## 返回值
///
/// 刷新成功返回 `Ok(())`；数据库错误返回 `Err`
pub async fn refresh_msg_queue_cache() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = get_db_conn()?;

    let queues: Vec<MsgMessageQueueVo> = MsgMessageQueueSvc::list_by_query_dto(
        MsgMessageQueueQueryDto::default(),
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();

    let queue_map: HashMap<String, QueueCache> = queues
        .into_iter()
        .map(|q| {
            (
                q.code.clone(),
                QueueCache {
                    id: *q.id,
                    code: q.code,
                    name: q.name,
                    persisted: q.persisted,
                },
            )
        })
        .collect();

    let unique_queues: HashSet<QueueCache> = queue_map.values().cloned().collect();

    MSG_QUEUE_CACHE.store(Some(Arc::new(queue_map)));

    // ── 同步队列订阅（队列变更时重新订阅） ──

    sync_queue_subscriptions(&unique_queues).await;

    Ok(())
}