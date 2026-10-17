//! # 消息缓存
//!
//! 将消息模板、渠道、队列、目标类别等低频变更数据从数据库加载到内存，
//! 利用 [`ArcSwapOption`] 实现无锁并发读取与原子热更新。
//!
//! ## 更新机制（robotech 本地缓存规范，对齐 dtw-rs）
//!
//! - **启动时**：`main.rs` 先调用 `setup_local_cache` 安装 JetStream 消费者，
//!   再调用 `refresh_msg_cache` / `refresh_queue_cache` 全量加载
//! - **运行时**：SVC 写操作成功后发布 JetStream 刷新信号，所有节点收到后全量重建
//! - **读取**：`ArcSwapOption::load_full()` 无锁并发读
//!
//! ```text
//! SVC write → after_write 回调 → JetStream publish 刷新信号 → 所有节点
//!   → DB 全量查询 → ArcSwapOption::store → 业务读取 get_msg_cache()
//! ```
//!
//! ## 数据结构
//!
//! - [`MessageCache`] — 单条消息的完整缓存条目
//! - [`ChannelCache`] — 渠道配置缓存
//!
//! ## 使用
//!
//! ```ignore
//! let cache = get_msg_cache();
//! if let Some(msg) = cache.get("order.created") {
//!     for ch in &msg.channels {
//!         // 直接用 ch.options / ch.remark 发送消息
//!     }
//! }
//! ```
//!
//! ## 参考
//!
//! 本模块结构与 [`dtw_rs::cache::dtw_local_cache`] 一致：
//! - `pub static` 全局缓存 → `build_*`（纯逻辑） / `refresh_*`（DB + build + store）
//! - `store_*` 原子替换到 `ArcSwapOption`
//! - `on_cache_refresh` 发布 JetStream 刷新信号
//! - `setup_local_cache` 由 `main.rs` 显式调用

use arc_swap::ArcSwapOption;
use msg_api::cst::{MSG_CACHE_REFRESH_SUBJECT, MSG_QUEUE_CACHE_REFRESH_SUBJECT};
use robotech::cache::publish_cache_refresh_signal;
use robotech::db::get_db_conn;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{error, info};

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

// ═══════════════════════════════════════════════════════════════
//  全局缓存
// ═══════════════════════════════════════════════════════════════

/// # 全局消息缓存
///
/// key = `msg_message.event_code`，value = [`MessageCache`]。
///
/// `ArcSwapOption::const_empty()` 是 `const fn`，可直接用于 `static` 初始化，
/// 无需 `LazyLock` 或 `Box::leak`。
///
/// `pub` 导出供 `main.rs` 通过 `setup_local_cache` 注册 JetStream 刷新信号消费者。
pub static MSG_CACHE: ArcSwapOption<HashMap<String, MessageCache>> =
    ArcSwapOption::const_empty();

/// # 全局消息队列缓存
///
/// 独立于消息缓存的队列缓存，用于订阅管理。
/// key = `queue.code`，value = [`QueueCache`]。
///
/// `pub` 导出供 `main.rs` 通过 `setup_local_cache` 注册独立的队列刷新消费者。
pub static MSG_QUEUE_CACHE: ArcSwapOption<HashMap<String, QueueCache>> = ArcSwapOption::const_empty();

// ═══════════════════════════════════════════════════════════════
//  数据结构
// ═══════════════════════════════════════════════════════════════

/// # 缓存的单条消息
///
/// 聚合了消息模板及其关联的类别、来源、渠道列表、目标列表。
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
//  公开 API — 存储
// ═══════════════════════════════════════════════════════════════

/// # 存储消息缓存（全量替换）
///
/// 与 [`refresh_msg_cache`] 配合，构建完成后调用此函数原子替换缓存快照。
/// 也供 `setup_local_cache` 的 JetStream 消费者回调内部使用：
/// `rebuild_fn` 返回的新缓存会通过 `ArcSwapOption::store` 落盘，
/// 因此本函数主要用于初始全量加载场景。
pub fn store_msg_cache(cache: HashMap<String, MessageCache>) {
    info!("存储消息缓存");
    MSG_CACHE.store(Some(Arc::new(cache)));
}

/// # 存储队列缓存（全量替换）
///
/// 仅替换缓存快照，不触发订阅同步（订阅同步由 [`refresh_queue_cache`] 在存储前完成）。
pub fn store_queue_cache(cache: HashMap<String, QueueCache>) {
    info!("存储消息队列缓存");
    MSG_QUEUE_CACHE.store(Some(Arc::new(cache)));
}

// ═══════════════════════════════════════════════════════════════
//  公开 API — 读取
// ═══════════════════════════════════════════════════════════════

/// # 获取全局消息缓存快照
///
/// 返回当前缓存快照的只读引用，调用方仅持有 [`Arc`] 共享所有权，无需拷贝。
/// 若缓存尚未初始化，返回空的默认缓存。
///
/// ## 返回值
///
/// [`Arc<HashMap<String, MessageCache>>`] — 当下的缓存快照
pub fn get_msg_cache() -> Arc<HashMap<String, MessageCache>> {
    MSG_CACHE
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()))
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

// ═══════════════════════════════════════════════════════════════
//  公开 API — 刷新信号
// ═══════════════════════════════════════════════════════════════

/// # 写操作后发布消息缓存刷新信号
///
/// 用于 `#[svc(after_write = ...)]`，任何消息相关配置写成功后通知所有节点全量重建。
pub async fn on_cache_refresh() {
    if let Err(e) = publish_cache_refresh_signal(MSG_CACHE_REFRESH_SUBJECT).await {
        error!("发布消息缓存刷新信号失败: {e}");
    }
}

/// # 写操作后发布队列缓存刷新信号
///
/// 队列变更需要同时刷新缓存并同步 NATS 订阅，因此使用独立主题，
/// 避免消息模板变更时触发无谓的重新订阅。
pub async fn on_queue_cache_refresh() {
    if let Err(e) = publish_cache_refresh_signal(MSG_QUEUE_CACHE_REFRESH_SUBJECT).await {
        error!("发布队列缓存刷新信号失败: {e}");
    }
}

// ═══════════════════════════════════════════════════════════════
//  公开 API — 全量刷新（DB 查询 + build + store）
// ═══════════════════════════════════════════════════════════════

/// # 全量刷新消息缓存
///
/// 从 DB 查询全量数据、组装 `HashMap`、存储到 [`MSG_CACHE`]，并返回结果。
///
/// 对应 dtw-rs 的 `refresh_all_control_point_groups`：
/// - `main.rs` 启动时调用做首次全量加载
/// - `setup_local_cache` 的 `rebuild_fn` 闭包也调用此函数，收到刷新信号后执行
pub async fn refresh_msg_cache() -> anyhow::Result<HashMap<String, MessageCache>> {
    info!("消息缓存全量加载...");
    let cache = build_msg_cache().await?;
    store_msg_cache(cache.clone());
    info!("消息缓存全量加载完成");
    Ok(cache)
}

/// # 全量刷新队列缓存
///
/// 从 DB 查询全量队列数据、同步 NATS 订阅、存储到 [`MSG_QUEUE_CACHE`]，并返回结果。
///
/// 队列缓存加载时会同步 NATS 订阅，因此消息缓存先于队列缓存加载，
/// 保证订阅到消息时消息模板已就绪。
pub async fn refresh_queue_cache() -> anyhow::Result<HashMap<String, QueueCache>> {
    info!("队列缓存全量加载...");
    let queues = rebuild_queue_cache().await?;
    store_queue_cache(queues.clone());
    info!("队列缓存全量加载完成");
    Ok(queues)
}

// ═══════════════════════════════════════════════════════════════
//  内部实现 — 消息缓存构建
// ═══════════════════════════════════════════════════════════════

/// # 从数据库构建消息缓存
///
/// 加载消息、渠道、来源、类别、目标类别、消息-目标关联，组装为 `HashMap<String, MessageCache>`。
/// 仅构建不落缓存，由调用方决定何时替换快照。
pub async fn build_msg_cache() -> anyhow::Result<HashMap<String, MessageCache>> {
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

    // ── 消息（通过 SVC list_ex 查询） ──

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
        .filter_map(|m| {
            let category = m
                .category_id
                .and_then(|id| categories.get(&id.0).cloned());
            let source = sources
                .get(&m.event_source_id.0)
                .cloned()
                .expect("消息事件来源应存在：event_source_id 为 NOT NULL FK");
            let channels = message_channels.get(&m.id.0).cloned().unwrap_or_default();
            let targets = message_targets.get(&m.id.0).cloned().unwrap_or_default();

            Some((
                m.event_code.clone(),
                MessageCache {
                    id: m.id.0,
                    event_code: m.event_code,
                    name: m.name,
                    persisted: m.persisted,
                    title_template: m.title_template,
                    content_template: m.content_template,
                    remark: m.remark,
                    category: category?,
                    source,
                    channels,
                    targets,
                },
            ))
        })
        .collect();

    Ok(messages)
}

// ═══════════════════════════════════════════════════════════════
//  内部实现 — 队列缓存重建
// ═══════════════════════════════════════════════════════════════

/// # 从数据库重建队列缓存并同步订阅
///
/// 返回最新的队列集合；缓存快照由调用方替换。
/// 队列的新增/移除会立即反映到 NATS 订阅上。
pub async fn rebuild_queue_cache() -> anyhow::Result<HashMap<String, QueueCache>> {
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

    // ── 同步队列订阅（队列变更时重新订阅） ──

    sync_queue_subscriptions(&unique_queues).await;

    Ok(queue_map)
}