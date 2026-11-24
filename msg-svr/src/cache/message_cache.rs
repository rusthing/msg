//! # 消息缓存
//!
//! 将消息模板、渠道、目标类别等低频变更数据从数据库加载到内存，
//! 利用 [`ArcSwapOption`] 实现无锁并发读取与原子热更新。
//!
//! ## 更新机制（robotech 刷新信号规范，对齐 dtw-rs）
//!
//! - **启动时**：`cache_utils::setup_cache` 先调用 `setup_local_cache` 注册刷新信号
//!   订阅，再调用 `refresh_message_cache` 全量加载
//! - **运行时**：SVC 写操作成功后发布 JetStream 刷新信号，所有节点收到后全量重建
//! - **读取**：`ArcSwapOption::load_full()` 无锁并发读
//!
//! ```text
//! SVC write → after_write 回调 → JetStream publish 刷新信号 → 所有节点
//!   → DB 全量查询 → ArcSwapOption::store → 业务读取 get_message_cache()
//! ```
//!
//! ## 队列不在缓存中
//!
//! 消息队列（`msg_message_queue`）只用于决定订阅哪些 subject，订阅状态由
//! `crate::sub` 自己维护，没有读取方需要队列快照。因此队列变更走另一条链路：
//!
//! ```text
//! 队列写操作 → 发布刷新信号 → 各节点 direct 查库 → 对比并增删 NATS 订阅
//! ```
//!
//! 详见 [`crate::sub::reload_message_queue_subscriptions`]；启动装配在
//! [`crate::utils::setup_message_queue_refresh`]。
//!
//! ## 数据结构
//!
//! - [`MessageCache`] — 单条消息的完整缓存条目
//! - [`ChannelCache`] — 渠道配置缓存
//!
//! ## 使用
//!
//! ```ignore
//! let cache = get_message_cache();
//! if let Some(msg) = cache.get("order.created") {
//!     for ch in &msg.channels {
//!         // 直接用 ch.options / ch.remark 发送消息
//!     }
//! }
//! ```
//!
//! ## 命名
//!
//! 本模块只负责「消息」这一份缓存，因此文件、静态、函数都带 `message`：
//! `message_cache.rs` / `MSG_MESSAGE_CACHE` / `get_message_cache` /
//! `refresh_message_cache` / `on_message_cache_refresh`。
//! 新增第二份缓存时新建 `{对象}_cache.rs`，不要去改这里的名字。
//!
//! ## 参考
//!
//! 本模块结构与 dtw-svr 的 `control_point_group_cache` 一致：
//! - `pub static` 全局缓存 → `build_{对象}_cache`（纯逻辑） / `refresh_{对象}_cache`（DB + build + store）
//! - `store_{对象}_cache` 原子替换到 `ArcSwapOption`
//! - `on_{对象}_cache_refresh` 发布 JetStream 刷新信号
//! - `setup_local_cache` 由 `cache_utils::setup_cache` 调用

use arc_swap::ArcSwapOption;
use msg_api::cst::{MSG_MESSAGE_REFRESH_SUBJECT, TARGET_ID_ALL};
use robotech::cache::publish_cache_refresh_signal;
use robotech::db::get_db_conn;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{error, info};

// ── SVC 引用 ──
use crate::dto::{
    MsgChannelQueryDto, MsgEventSourceQueryDto, MsgMessageCategoryQueryDto, MsgMessageQueryDto,
    MsgMessageTargetChannelQueryDto, MsgMessageTargetQueryDto, MsgTargetCategoryChannelQueryDto,
    MsgTargetCategoryQueryDto,
};
use crate::svc::{
    MsgChannelSvc, MsgEventSourceSvc, MsgMessageCategorySvc, MsgMessageSvc,
    MsgMessageTargetChannelSvc, MsgMessageTargetSvc, MsgTargetCategoryChannelSvc,
    MsgTargetCategorySvc,
};
use crate::vo::{
    MsgChannelVo, MsgMessageExVo, MsgMessageTargetChannelVo, MsgMessageTargetVo,
    MsgTargetCategoryChannelVo,
};

// ═══════════════════════════════════════════════════════════════
//  全局缓存
// ═══════════════════════════════════════════════════════════════

/// # 全局消息缓存
///
/// key = `msg_message.event_code`，value = [`MessageCache`]。
///
/// 名字带对象（`MESSAGE`）而不是笼统的 `MSG_CACHE`：一个服务可能有多份缓存，
/// 对象段需与刷新主题 `refresh.msg.message` 的末段对应，新增第二份缓存时
/// 才知道该往哪里插名字（`MSG_QUEUE_CACHE`、`MSG_TEMPLATE_CACHE` …）。
///
/// `ArcSwapOption::const_empty()` 是 `const fn`，可直接用于 `static` 初始化，
/// 无需 `LazyLock` 或 `Box::leak`。
///
/// `pub` 导出供 `cache_utils::setup_cache` 通过 `setup_local_cache` 注册
/// JetStream 刷新信号消费者。
pub static MSG_MESSAGE_CACHE: ArcSwapOption<HashMap<String, MessageCache>> =
    ArcSwapOption::const_empty();

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
    /// 是否允许模拟通知投递（默认关）
    ///
    /// 关闭时带 `simulated=true` 标记的消息在投递前被丢弃。
    pub sim_enabled: bool,
    /// 标题模板（为 null 时用发布方给的标题）
    pub title_template: Option<String>,
    /// 内容模板（为 null 时用发布方给的内容）
    pub content_template: Option<String>,
    /// 附加内容模板（为 null 时用发布方给的附加内容）
    pub ext_template: Option<String>,
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
    /// 渠道类型：渠道唯一标识，决定用哪个发送器、options 按哪套结构解析
    /// （见 `ChannelCode`），同时是投递主题 `msg.delivery.{channel_type}` 的末段
    pub channel_type: String,
    /// 名称
    pub name: String,
    /// 渠道选项（JSON 字符串）
    pub options: Option<String>,
    /// 载荷模板（为空时用发送器内置的默认报文结构）
    pub payload_template: Option<String>,
    /// 署名
    pub remark: Option<String>,
}

/// # 缓存的目标类别
///
/// 分类标签 + 该类别 owner 声明的业务规则：`allow_broadcast` / `target_keys`
/// 决定目标怎么展开；类别对应的默认渠道（`msg_target_category_channel`）在
/// 「消息目标没有显式渠道」时兜底，见 [`resolve_target_channels`]。
#[derive(Debug, Clone)]
pub struct TargetCategoryCache {
    /// 类别 ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 是否允许广播：false 时该类别必须给出具体 target_id
    pub allow_broadcast: bool,
    /// 目标标识键：从消息 labels 取目标标识，空表示只用 target_id
    pub target_keys: Option<String>,
}

/// # 缓存的消息目标
///
/// 直接内嵌所属类别，并把该目标要走的渠道组装好：`msg_message_target_channel`
/// 里显式配的优先，没配就回退到该类别的默认渠道。
/// 投递时只看 `target.channels` —— 这是渠道的唯一来源。
#[derive(Debug, Clone)]
pub struct TargetCache {
    /// 该目标的 ID（`msg_message_target.id`）
    pub id: u64,
    pub target_category: TargetCategoryCache,
    /// 目标标识；空串表示该目标类别下的全部目标（按类别广播）
    pub target_id: String,
    /// 该目标要走的渠道列表（含完整配置，刷新时组装）
    ///
    /// 显式绑定优先，没显式绑定就是类别默认渠道。为空说明两边都没配，
    /// 投递侧会报错跳过（不会静默成功）。
    pub channels: Vec<ChannelCache>,
}

// ═══════════════════════════════════════════════════════════════
//  内部实现 — 渠道解析
// ═══════════════════════════════════════════════════════════════

/// 把渠道查询结果转成缓存结构
fn to_channel_cache(ch: &MsgChannelVo) -> ChannelCache {
    ChannelCache {
        id: ch.id.0,
        channel_type: ch.channel_type.clone(),
        name: ch.name.clone(),
        options: ch.options.clone(),
        payload_template: ch.payload_template.clone(),
        remark: ch.remark.clone(),
    }
}

/// 取某个消息目标要走的渠道（含完整配置）
///
/// 取渠道的顺序是两级：
///
/// 1. `msg_message_target_channel` 里显式绑定的渠道——这条消息的这个目标单独指定；
/// 2. 没显式绑定就回退到 `msg_target_category_channel` 里该类别的默认渠道（类别
///    owner 注册上来的出口，`is_default` 标记；一条都没标时取该类别的全部绑定）。
///
/// 这样注册方只要登记「消息 → 类别」和「类别 → 渠道」，不必再为每条消息补一遍
/// 消息目标的渠道行。渠道本身 `enabled=false` 时不会出现在 `channels_by_id` 里，
/// 等价于没配。
fn resolve_target_channels(
    message_target_channels: &HashMap<u64, Vec<u64>>,
    category_channels: &HashMap<u64, Vec<u64>>,
    message_target_id: u64,
    category_id: u64,
    channels_by_id: &HashMap<u64, &MsgChannelVo>,
) -> Vec<ChannelCache> {
    let ch_ids = message_target_channels
        .get(&message_target_id)
        .filter(|ids| !ids.is_empty())
        .or_else(|| category_channels.get(&category_id))
        .cloned()
        .unwrap_or_default();
    ch_ids
        .iter()
        .filter_map(|ch_id| channels_by_id.get(ch_id).copied())
        .map(to_channel_cache)
        .collect()
}

// ═══════════════════════════════════════════════════════════════
//  公开 API — 存储
// ═══════════════════════════════════════════════════════════════

/// # 存储消息缓存（全量替换）
///
/// 与 [`refresh_message_cache`] 配合，构建完成后调用此函数原子替换缓存快照。
/// 也供 `setup_local_cache` 的 JetStream 消费者回调内部使用：
/// `rebuild_fn` 返回的新缓存会通过 `ArcSwapOption::store` 落盘，
/// 因此本函数主要用于初始全量加载场景。
pub fn store_message_cache(cache: HashMap<String, MessageCache>) {
    info!("存储消息缓存");
    MSG_MESSAGE_CACHE.store(Some(Arc::new(cache)));
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
pub fn get_message_cache() -> Arc<HashMap<String, MessageCache>> {
    MSG_MESSAGE_CACHE
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()))
}

// ═══════════════════════════════════════════════════════════════
//  公开 API — 刷新信号
// ═══════════════════════════════════════════════════════════════

/// # 写操作后发布消息缓存刷新信号
///
/// 用于 `#[svc(after_write = ...)]`，任何消息相关配置写成功后通知所有节点全量重建。
///
/// 命名带对象前缀（`message`），与主题 `refresh.msg.message` 的末段对应；
/// 同一服务新增缓存时按 `on_{对象}_cache_refresh` 命名，避免笼统的
/// `on_cache_refresh` 无法区分是哪一份缓存。
pub async fn on_message_cache_refresh() {
    if let Err(e) = publish_cache_refresh_signal(&MSG_MESSAGE_REFRESH_SUBJECT).await {
        error!("发布消息缓存刷新信号失败: {e}");
    }
}

// ═══════════════════════════════════════════════════════════════
//  公开 API — 全量刷新（DB 查询 + build + store）
// ═══════════════════════════════════════════════════════════════

/// # 全量刷新消息缓存
///
/// 从 DB 查询全量数据、组装 `HashMap`、存储到 [`MSG_MESSAGE_CACHE`]，并返回结果。
///
/// 对应 dtw-rs 的 `refresh_control_point_group_cache`：
/// - `cache_utils::setup_cache` 启动时调用做首次全量加载
/// - `setup_local_cache` 的 `rebuild_fn` 闭包也调用此函数，收到刷新信号后执行
pub async fn refresh_message_cache() -> anyhow::Result<HashMap<String, MessageCache>> {
    info!("消息缓存全量加载...");
    let cache = build_message_cache().await?;
    store_message_cache(cache.clone());
    info!("消息缓存全量加载完成");
    Ok(cache)
}

// ═══════════════════════════════════════════════════════════════
//  内部实现 — 消息缓存构建
// ═══════════════════════════════════════════════════════════════

/// # 从数据库构建消息缓存
///
/// 加载消息、渠道、来源、类别、目标类别、消息-目标关联，组装为 `HashMap<String, MessageCache>`。
/// 仅构建不落缓存，由调用方决定何时替换快照。
pub async fn build_message_cache() -> anyhow::Result<HashMap<String, MessageCache>> {
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

    // ── 目标类别（只做分类标签，渠道不在这里） ──

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
            (
                m.id.0,
                TargetCategoryCache {
                    id: m.id.0,
                    code: m.code,
                    name: m.name,
                    allow_broadcast: m.allow_broadcast,
                    target_keys: m.target_keys,
                },
            )
        })
        .collect();

    // ── 目标类别 → 默认渠道（消息目标没显式配渠道时的回退出口） ──
    //
    // 类别 owner（如 dtw）首次启动注册的就是这里：类别 + 渠道 + `is_default`。
    // 一个类别标了默认的只留默认那几条，一条都没标就留全部绑定——历史数据里
    // 没有 `is_default` 的类别仍按「全部绑定」投。
    let category_channels: HashMap<u64, Vec<u64>> = {
        let links: Vec<MsgTargetCategoryChannelVo> =
            MsgTargetCategoryChannelSvc::list_by_query_dto(
                MsgTargetCategoryChannelQueryDto::default(),
                Some(db.as_ref()),
            )
            .await?
            .extra
            .unwrap_or_default();
        let mut all: HashMap<u64, Vec<u64>> = HashMap::new();
        let mut defaults: HashMap<u64, Vec<u64>> = HashMap::new();
        for link in links {
            let category_id = link.target_category_id.0;
            let channel_id = link.channel_id.0;
            all.entry(category_id).or_default().push(channel_id);
            if link.is_default {
                defaults.entry(category_id).or_default().push(channel_id);
            }
        }
        all.into_iter()
            .map(|(category_id, channel_ids)| {
                let chosen = defaults.remove(&category_id).unwrap_or(channel_ids);
                (category_id, chosen)
            })
            .collect()
    };

    // ── 消息目标 → 渠道（显式绑定优先于类别默认渠道） ──

    let message_target_channels: HashMap<u64, Vec<u64>> = {
        let links: Vec<MsgMessageTargetChannelVo> = MsgMessageTargetChannelSvc::list_by_query_dto(
            MsgMessageTargetChannelQueryDto::default(),
            Some(db.as_ref()),
        )
        .await?
        .extra
        .unwrap_or_default();
        let mut map: HashMap<u64, Vec<u64>> = HashMap::new();
        for link in links {
            map.entry(link.message_target_id.0)
                .or_default()
                .push(link.channel_id.0);
        }
        map
    };

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

    // ── 消息-渠道关联（该消息各目标的渠道并集，去重） ──

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
            for ch in resolve_target_channels(
                &message_target_channels,
                &category_channels,
                link.id.0,
                link.target_category_id.0,
                &channels_by_id,
            ) {
                let entry = map.entry(link.message_id.0).or_default();
                if !entry.iter().any(|c| c.id == ch.id) {
                    entry.push(ch);
                }
            }
        }
        map
    };

    // ── 消息-目标关联（复用 target_categories） ──

    // 目标配了却没有任何渠道的，缓存里照样留着（投递侧报错跳过），组装完统一报一次
    let mut empty_channel_targets: Vec<(u64, String, String)> = Vec::new();

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
                let channels = resolve_target_channels(
                    &message_target_channels,
                    &category_channels,
                    link.id.0,
                    link.target_category_id.0,
                    &channels_by_id,
                );
                if channels.is_empty() {
                    empty_channel_targets.push((
                        link.message_id.0,
                        tc.code.clone(),
                        link.target_id.clone(),
                    ));
                }
                map.entry(link.message_id.0).or_default().push(TargetCache {
                    id: link.id.0,
                    target_category: tc.clone(),
                    target_id: link.target_id,
                    channels,
                });
            }
        }
        // 同一个类别下如果登记了广播（target_id 为空串），具体的那些目标就是多余的：
        // 广播已经把该类别下的所有目标都覆盖了，留着只会让那几块屏收两遍。
        // 在缓存里就地剔掉，投递侧不用再判断。
        for targets in map.values_mut() {
            let broadcast_categories: HashSet<u64> = targets
                .iter()
                .filter(|target| target.target_id == TARGET_ID_ALL)
                .map(|target| target.target_category.id)
                .collect();
            if broadcast_categories.is_empty() {
                continue;
            }
            targets.retain(|target| {
                target.target_id == TARGET_ID_ALL
                    || !broadcast_categories.contains(&target.target_category.id)
            });
        }
        map
    };

    if !empty_channel_targets.is_empty() {
        error!(
            "有消息目标未配渠道，这些目标不会产生任何投递任务: 共 {} 条，示例(消息ID, 类别编码, 目标标识): {:?}",
            empty_channel_targets.len(),
            &empty_channel_targets[..empty_channel_targets.len().min(10)]
        );
    }

    // ── 组装消息 ──

    let messages: HashMap<String, MessageCache> = messages_all
        .into_iter()
        .map(|m| {
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
                    sim_enabled: m.sim_enabled,
                    title_template: m.title_template,
                    content_template: m.content_template,
                    ext_template: m.ext_template,
                    remark: m.remark,
                    category,
                    source,
                    channels,
                    targets,
                },
            )
        })
        .collect();

    Ok(messages)
}
