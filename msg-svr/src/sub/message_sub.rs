//! # 队列订阅管理
//!
//! 维护当前活跃的消息队列订阅句柄，队列变更时根据最新的队列信息同步订阅
//! 状态：新增队列则启动新订阅，移除的队列则停止旧的订阅线程。
//!
//! ## 数据流
//!
//! ```text
//! 队列变更信号（启动加载 / JetStream 队列刷新信号）
//!   → 从 DB 查询所有 msg_message_queue
//!   → 映射为 QueueSpec（订阅只需要 code + persisted）
//!   → sync_queue_subscriptions(specs)
//!     → 对比旧订阅：新增 / 保留 / 移除
//!     → 调用 NATS subscribe 启动新线程
//!     → 停止不再需要的订阅线程
//! ```
//!
//! 队列信息**不进缓存**：它唯一的用途是决定订阅哪些 subject，
//! 而订阅状态本身由 [`QUEUE_SUBSCRIBERS`] 维护。缓存一份队列数据只会
//! 多一份需要同步的快照，没有任何读取方。
//!
//! 注意：消息模板缓存（`build_message_cache`）不会触发队列重新订阅。

use crate::cache::{MessageCache, get_message_cache};
use crate::config::get_msg_config;
use crate::dto::MsgMessageQueueQueryDto;
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc, MsgMessageQueueSvc};
use crate::utils::render_template;
use crate::vo::MsgMessageQueueVo;
use arc_swap::ArcSwapOption;
use idworker::next_id;
use msg_api::cst::MSG_QUEUE_REFRESH_SUBJECT;
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::dto::{
    MsgDeliveryAddDto, MsgDeliveryChannelAddDto, MsgDeliveryQueryDto, MsgDeliveryTargetAddDto,
};
use msg_api::mqo::message_mqo::MessageMqo;
use robotech::api::U64;
use robotech::db::get_db_conn;
use robotech::mq::nats;
use robotech::mq::nats::NatsError;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// # 队列订阅规格
///
/// 订阅层只关心「订阅哪个 subject、是否持久化」，
/// 不需要完整的队列实体。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueueSpec {
    /// 队列编码，同时作为 NATS subject
    pub code: String,
    /// 是否持久化（`true` → JetStream，`false` → Core NATS）
    pub persisted: bool,
}

/// # 队列订阅句柄
///
/// 除了订阅任务句柄，还记录「建立订阅时应用的重投上限」：配置热更新改了
/// `msg.max-deliver` 之后，靠这个值判断哪些 consumer 需要重建。
#[derive(Debug)]
struct QueueSubscription {
    /// 订阅任务句柄
    handle: Arc<JoinHandle<()>>,
    /// 是否 JetStream 订阅（Core NATS 没有 consumer，重投上限对它无意义）
    persisted: bool,
    /// 建立订阅时应用的重投上限
    max_deliver: i64,
}

/// # 全局队列订阅句柄
///
/// key = `queue.code`（即 NATS subject），value = 订阅线程句柄。
/// 使用 [`ArcSwapOption`] 实现无锁热替换。
static QUEUE_SUBSCRIBERS: ArcSwapOption<HashMap<String, QueueSubscription>> =
    ArcSwapOption::const_empty();

/// # 重新加载队列订阅
///
/// 从 DB 查询全量队列并同步订阅。启动时调用一次，之后每次收到队列刷新
/// 信号都调用——这是队列变更后唯一的处理入口，没有缓存快照需要维护。
pub async fn reload_queue_subscriptions() -> anyhow::Result<()> {
    info!("队列订阅同步...");

    let db = get_db_conn()?;
    let queues: Vec<MsgMessageQueueVo> = MsgMessageQueueSvc::list_by_query_dto(
        MsgMessageQueueQueryDto::default(),
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();

    let specs: HashSet<QueueSpec> = queues
        .into_iter()
        .map(|q| QueueSpec {
            code: q.code,
            persisted: q.persisted,
        })
        .collect();

    sync_queue_subscriptions(&specs).await;
    info!("队列订阅同步完成");
    Ok(())
}

/// # 发布队列刷新信号
///
/// 用于 `#[svc(after_write = ...)]`：队列配置写成功后通知所有节点同步订阅。
pub async fn on_queue_refresh() {
    if let Err(e) = robotech::refresh::publish(&MSG_QUEUE_REFRESH_SUBJECT).await {
        error!("发布队列刷新信号失败: {e}");
    }
}

/// # 同步队列订阅
///
/// 根据给定的队列集合，对比当前活跃订阅：
/// - 队列 `code` 不存在于当前订阅 → 启动新订阅
/// - 队列 `code` 已存在且订阅任务仍在运行 → 保留（跳过）
/// - 队列 `code` 已存在但订阅任务**已退出** → 视为不存在，重建
/// - 队列 `code` 已存在但**重投上限已变更** → 停掉旧订阅，按新上限重建
/// - 当前订阅中的 `code` 不在 `queues` 中 → 停止
///
/// 「任务已退出」必须算作不存在：连接关闭（如 NATS 配置热更新换掉 Client）
/// 会让订阅任务自行结束，但句柄仍留在 [`QUEUE_SUBSCRIBERS`] 里。只比 `code`
/// 集合的话，这些死订阅永远不会被重建。
///
/// 「重投上限已变更」必须重建：`get_or_create_consumer` 对已存在的 consumer
/// 不会应用新配置，只有重新 `subscribe` 才会走到 `update_consumer`，
/// 把 `msg.max-deliver` 的新值原地刷到服务器上。
///
/// 每个队列根据 `persisted` 字段自动选择 Core NATS（`false`）或
/// JetStream Push（`true`）订阅模式。
///
/// ## 参数
///
/// - `queues`: 需要保持订阅的队列规格集合
pub async fn sync_queue_subscriptions(queues: &HashSet<QueueSpec>) {
    let current = QUEUE_SUBSCRIBERS
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()));

    let new_codes: HashSet<String> = queues.iter().map(|q| q.code.clone()).collect();

    // 重投上限来自配置文件，改配置后热更新会重新走到这里
    let max_deliver = get_msg_config().effective_max_deliver();

    // ── 保留仍然存活的订阅，死句柄/配置已变的直接丢弃（下面会按新增重建）──

    let mut next: HashMap<String, QueueSubscription> = HashMap::new();
    let mut live_codes: HashSet<String> = HashSet::new();

    for (code, subscription) in current.iter() {
        if subscription.handle.is_finished() {
            warn!("队列订阅任务已退出，将重建: {code}");
            continue;
        }
        if subscription.persisted && subscription.max_deliver != max_deliver {
            info!(
                "队列重投上限变更（{} → {max_deliver}），重建订阅: {code}",
                subscription.max_deliver
            );
            subscription.handle.abort();
            continue;
        }
        next.insert(
            code.clone(),
            QueueSubscription {
                handle: Arc::clone(&subscription.handle),
                persisted: subscription.persisted,
                max_deliver: subscription.max_deliver,
            },
        );
        live_codes.insert(code.clone());
    }

    // ── 停止已移除的队列订阅 ──

    for code in live_codes.difference(&new_codes) {
        if let Some(subscription) = next.remove(code) {
            info!("停止队列订阅: {code}");
            subscription.handle.abort();
        }
    }

    // ── 启动新增（含上一轮已退出 / 重投上限已变）的队列订阅 ──

    for code in new_codes.difference(&live_codes) {
        // 默认不持久化（Core NATS）
        let persisted = queues
            .iter()
            .find(|q| q.code == *code)
            .map(|q| q.persisted)
            .unwrap_or(false);

        let handle = match start_subscription_for_queue(code, persisted, max_deliver).await {
            Ok(handle) => {
                info!("启动队列订阅成功: {code}");
                handle
            }
            Err(e) => {
                error!("启动队列订阅失败 {code}: {e}");
                continue;
            }
        };

        next.insert(
            code.clone(),
            QueueSubscription {
                handle,
                persisted,
                max_deliver,
            },
        );
    }

    QUEUE_SUBSCRIBERS.store(Some(Arc::new(next)));
}

/// # 队列组名
///
/// 同一队列的所有 msg-svr 副本必须共用同一个组名，否则同一条消息会被
/// 每个副本各收一份（普通订阅语义是「所有订阅者都收到」）。
///
/// - JetStream 模式：写入 consumer 的 `deliver_group`
/// - Core NATS 模式：作为 `queue_subscribe` 的组名
const QUEUE_GROUP: &str = "msg-svr";

/// # 为指定队列启动 NATS 订阅
///
/// - `persisted = false` → Core NATS 订阅（at-most-once）
/// - `persisted = true` → JetStream Push 订阅，创建的 Stream 名称为 `MSG-{code}`，
///   Consumer 名称为 `msg-svr-{code}`
/// - `max_deliver` → 消费失败的最大重投次数，来自配置 `msg.max-deliver`
///
/// 两种模式都加入 [`QUEUE_GROUP`] 队列组，由服务端在副本之间负载均衡。
async fn start_subscription_for_queue(
    subject: &str,
    persisted: bool,
    max_deliver: i64,
) -> Result<Arc<JoinHandle<()>>, Box<dyn std::error::Error + Send + Sync>> {
    use async_nats::jetstream::{consumer, stream};

    let owned_subject = subject.to_string();

    if persisted {
        let stream_name = format!("MSG-{subject}");
        let stream_config = stream::Config {
            name: stream_name.clone(),
            subjects: vec![subject.to_string()],
            // 保留 7 天：消费端长时间停机后仍能补上，同时避免 stream 无限增长
            // （`max_age` 默认 0 表示永久保留，文件存储会一直占盘）
            max_age: Duration::from_secs(7 * 24 * 3600),
            ..Default::default()
        };

        let consumer_config = consumer::push::Config {
            durable_name: Some(format!("msg-svr-{subject}")),
            deliver_policy: consumer::DeliverPolicy::All,
            ack_policy: consumer::AckPolicy::Explicit,
            // 处理失败的消息最多重投 max_deliver 次，避免毒消息无限循环
            max_deliver,
            ..Default::default()
        };

        nats::subscribe(
            subject,
            Some(QUEUE_GROUP),
            Some(stream_config),
            Some(consumer_config),
            move |msg| {
                let subj = owned_subject.clone();
                async move {
                    info!(
                        "收到 JetStream 消息 [{}]: {}",
                        subj,
                        String::from_utf8_lossy(&msg.payload)
                    );
                    process_message(&subj, &msg.payload).await
                }
            },
        )
        .await
        .map_err(Into::into)
    } else {
        nats::subscribe(subject, Some(QUEUE_GROUP), None, None, move |msg| {
            let subj = owned_subject.clone();
            async move {
                info!(
                    "收到 Core NATS 消息 [{}]: {}",
                    subj,
                    String::from_utf8_lossy(&msg.payload)
                );
                process_message(&subj, &msg.payload).await
            }
        })
        .await
        .map_err(Into::into)
    }
}

/// 处理收到的 NATS 消息
///
/// ## 处理流程
///
/// 1. 解析 JSON 负载
/// 2. 从缓存查找消息模板（按 `event_code`）
/// 3. 校验队列匹配
/// 4. 渲染标题和内容（模板用 labels 替换；若模板为 null 则从 annotations 取）
/// 5. 幂等检查（`business_id` 唯一约束）
/// 6. 创建 `msg_delivery` 投递记录（含 labels / annotations JSON）
/// 7. 为每个目标创建 `msg_delivery_target`
/// 8. 为每个（目标 × 渠道）组合创建 `msg_delivery_channel`
///
/// ## 返回值
///
/// - `Ok(())`：消息处理成功（含幂等跳过）
/// - `Err(NatsError)`：处理失败
///
/// JetStream 模式下 `Ok(())` 会触发 ACK，`Err` 会导致消息被重新投递。
pub async fn process_message(subject: &str, payload: &[u8]) -> Result<(), NatsError> {
    // ── 1. 解析消息负载 ──

    let message_mqo: MessageMqo = serde_json::from_slice(payload)
        .map_err(|e| NatsError::Discard(format!("消息负载 JSON 解析失败 [{}]: {}", subject, e)))?;

    info!(
        "收到消息: event_code={}, business_id={}, subject={}",
        message_mqo.event_code, message_mqo.business_id, subject
    );

    // ── 2. 查找消息模板 ──

    let cache = get_message_cache();
    let msg: &MessageCache = cache.get(&message_mqo.event_code).ok_or_else(|| {
        NatsError::Handle(format!(
            "未找到事件编码对应的消息模板: {}",
            message_mqo.event_code
        ))
    })?;

    // ── 3. 渲染标题和内容 ──
    //
    //   title_template 不为 null → 用 labels 做变量替换，替换 title_template
    //   title_template 为 null    → 取 annotations["title"]
    //
    //   content_template 不为 null → 用 labels 做变量替换，替换 content_template
    //   content_template 为 null    → 取 annotations["content"]

    let title = match &msg.title_template {
        Some(tpl) => render_template(tpl, &message_mqo.labels),
        None => message_mqo
            .annotations
            .get("title")
            .cloned()
            .unwrap_or_default(),
    };

    let content = match &msg.content_template {
        Some(tpl) => render_template(tpl, &message_mqo.labels),
        None => message_mqo
            .annotations
            .get("content")
            .cloned()
            .unwrap_or_default(),
    };

    if title.is_empty() && content.is_empty() {
        // 模板和 annotations 都取不到内容，重投多少次都一样
        return Err(NatsError::Discard("标题和内容均为空，无法投递".into()));
    }

    info!("内容渲染完成: title={title}");

    // ── 4. 幂等检查 ──

    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;

    let existing = MsgDeliverySvc::get_by_query_dto(
        MsgDeliveryQueryDto {
            business_id: Some(message_mqo.business_id.into()),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await;

    match existing {
        Ok(ro) if ro.extra.is_some() => {
            info!(
                "消息已投递，幂等跳过: business_id={}",
                message_mqo.business_id
            );
            return Ok(());
        }
        Ok(_) => {}
        Err(e) => {
            warn!("幂等检查查询失败，继续处理: {e}");
        }
    }

    // ── 5. 创建投递记录 ──

    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let delivery_id = next_id().map_err(|e| NatsError::Handle(format!("生成投递ID失败: {e}")))?;

    let labels_json = if message_mqo.labels.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&message_mqo.labels)
                .map_err(|e| NatsError::Handle(format!("labels 序列化失败: {e}")))?,
        )
    };

    let annotations_json = if message_mqo.annotations.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&message_mqo.annotations)
                .map_err(|e| NatsError::Handle(format!("annotations 序列化失败: {e}")))?,
        )
    };

    let delivery_add = MsgDeliveryAddDto {
        id: Some(U64(delivery_id)),
        event_code: Some(msg.event_code.clone()),
        message_id: Some(msg.id.into()),
        event_source_id: Some(msg.source.id.into()),
        message_category_id: Some(msg.category.id.into()),
        business_id: Some(message_mqo.business_id.into()),
        business_trigger_ms: Some(
            message_mqo
                .business_trigger_ms
                .unwrap_or(now_ms)
                .into(),
        ),
        deliver_status: Some(DeliverStatus::Delivering),
        labels: Some(labels_json),
        annotations: Some(annotations_json),
        title: Some(title),
        content: Some(content),
        remark: None,
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
    };

    MsgDeliverySvc::add(delivery_add, Some(db.as_ref()))
        .await
        .map_err(|e| {
            NatsError::Handle(format!(
                "创建投递记录失败 (business_id={}): {e}",
                message_mqo.business_id
            ))
        })?;

    info!(
        "投递记录已创建: delivery_id={delivery_id}, message_id={}",
        msg.id
    );

    // ── 6. 创建投递目标 ──

    for target in &msg.targets {
        let target_id_val =
            next_id().map_err(|e| NatsError::Handle(format!("生成投递目标ID失败: {e}")))?;

        let target_add = MsgDeliveryTargetAddDto {
            id: Some(U64(target_id_val)),
            delivery_id: Some(delivery_id.into()),
            target_category_id: Some(target.target_category.id.into()),
            target_id: Some(target.target_id.into()),
            deliver_target_status: Some(DeliverTargetStatus::Delivering),
            _current_ms: Some(U64(now_ms)),
            _current_user_id: U64(0),
        };

        match MsgDeliveryTargetSvc::add(target_add, Some(db.as_ref())).await {
            Ok(ro) => {
                let created_target = ro.extra.as_ref().unwrap();
                info!(
                    "投递目标已创建: target_id={}, category_code={}, delivery_id={delivery_id}",
                    created_target.id, target.target_category.code
                );

                // ── 7. 直接用目标内嵌的渠道列表创建投递渠道记录 ──

                for channel in &target.target_category.channels {
                    let channel_id_val = next_id()
                        .map_err(|e| NatsError::Handle(format!("生成投递渠道ID失败: {e}")))?;

                    let channel_add = MsgDeliveryChannelAddDto {
                        id: Some(U64(channel_id_val)),
                        deliver_target_id: Some(created_target.id),
                        channel_id: Some(channel.id.into()),
                        deliver_channel_status: Some(DeliverChannelStatus::Delivering),
                        address: None,
                        _current_ms: Some(U64(now_ms)),
                        _current_user_id: U64(0),
                    };

                    if let Err(e) = MsgDeliveryChannelSvc::add(channel_add, Some(db.as_ref())).await
                    {
                        error!(
                            "创建投递渠道失败: target_id={}, channel_id={}, error={e}",
                            created_target.id, channel.id
                        );
                    } else {
                        info!(
                            "投递渠道已创建: target_id={}, channel_id={}, channel_name={}",
                            created_target.id, channel.id, channel.name
                        );
                    }
                }
            }
            Err(e) => {
                error!(
                    "创建投递目标失败: category_code={}, error={e}",
                    target.target_category.code
                );
            }
        }
    }

    info!(
        "消息处理完成: event_code={}, business_id={}",
        message_mqo.event_code, message_mqo.business_id
    );

    Ok(())
}