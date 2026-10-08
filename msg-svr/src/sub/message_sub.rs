//! # 队列订阅管理
//!
//! 维护当前活跃的消息队列订阅句柄，在队列缓存刷新时根据最新的队列信息
//! 同步订阅状态：新增队列则启动新订阅，移除的队列则停止旧的订阅线程。
//!
//! ## 数据流
//!
//! ```text
//! refresh_msg_queue_cache
//!   → 从 DB 加载所有 msg_message_queue
//!   → 提取所有 CachedQueue
//!   → sync_queue_subscriptions(queues)
//!     → 对比旧订阅：新增 / 保留 / 移除
//!     → 调用 NATS subscribe 启动新线程
//!     → 停止不再需要的订阅线程
//! ```
//!
//! 注意：消息模板缓存（`refresh_msg_message_cache`）不会触发队列重新订阅。

use crate::cache::{MessageCache, QueueCache, get_msg_cache};
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc};
use crate::utils::render_template;
use arc_swap::ArcSwapOption;
use idworker::next_id;
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
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// # 全局队列订阅句柄
///
/// key = `queue.code`（即 NATS subject），value = 订阅线程句柄。
/// 使用 [`ArcSwapOption`] 实现无锁热替换。
static QUEUE_SUBSCRIBERS: ArcSwapOption<HashMap<String, Arc<JoinHandle<()>>>> =
    ArcSwapOption::const_empty();

/// # 同步队列订阅
///
/// 根据给定的队列集合，对比当前活跃订阅：
/// - 队列 `code` 不存在于当前订阅 → 启动新订阅
/// - 队列 `code` 已存在 → 保留（跳过）
/// - 当前订阅中的 `code` 不在 `queues` 中 → 停止
///
/// 每个队列根据 `persisted` 字段自动选择 Core NATS（`false`）或
/// JetStream Push（`true`）订阅模式。
///
/// ## 参数
///
/// - `queues`: 从当前缓存中提取的所有唯一消息队列列表
pub async fn sync_queue_subscriptions(queues: &HashSet<QueueCache>) {
    let current = QUEUE_SUBSCRIBERS
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()));

    let current_codes: HashSet<String> = current.keys().cloned().collect();
    let new_codes: HashSet<String> = queues.iter().map(|q| q.code.clone()).collect();

    // ── 停止已移除的队列订阅 ──

    let removed: HashSet<&String> = current_codes.difference(&new_codes).collect();
    let mut next = HashMap::new();

    for (code, handle) in current.iter() {
        if removed.contains(code) {
            info!("停止队列订阅: {code}");
            handle.abort();
        } else {
            next.insert(code.clone(), Arc::clone(handle));
        }
    }

    // ── 启动新增的队列订阅 ──

    let added: HashSet<&String> = new_codes.difference(&current_codes).collect();

    for &code in &added {
        // 默认不持久化（Core NATS）
        let persisted = queues
            .iter()
            .find(|q| q.code == *code)
            .map(|q| q.persisted)
            .unwrap_or(false);

        let handle = match start_subscription_for_queue(code, persisted).await {
            Ok(handle) => {
                info!("启动队列订阅成功: {code}");
                handle
            }
            Err(e) => {
                error!("启动队列订阅失败 {code}: {e}");
                continue;
            }
        };

        next.insert(code.clone(), handle);
    }

    if !added.is_empty() || !removed.is_empty() || QUEUE_SUBSCRIBERS.load_full().is_none() {
        QUEUE_SUBSCRIBERS.store(Some(Arc::new(next)));
    }
}

/// # 为指定队列启动 NATS 订阅
///
/// - `persisted = false` → Core NATS 订阅（at-most-once）
/// - `persisted = true` → JetStream Push 订阅，创建的 Stream 名称为 `MSG-{code}`，
///   Consumer 名称为 `msg-svr-{code}`
async fn start_subscription_for_queue(
    subject: &str,
    persisted: bool,
) -> Result<Arc<JoinHandle<()>>, Box<dyn std::error::Error + Send + Sync>> {
    use async_nats::jetstream::{consumer, stream};

    let owned_subject = subject.to_string();

    if persisted {
        let stream_name = format!("MSG-{subject}");
        let stream_config = stream::Config {
            name: stream_name.clone(),
            subjects: vec![subject.to_string()],
            ..Default::default()
        };

        let consumer_config = consumer::push::Config {
            durable_name: Some(format!("msg-svr-{subject}")),
            deliver_policy: consumer::DeliverPolicy::All,
            ack_policy: consumer::AckPolicy::Explicit,
            ..Default::default()
        };

        nats::subscribe(
            subject,
            None,
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
        nats::subscribe(subject, None, None, None, move |msg| {
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
        .map_err(|e| NatsError::Handle(format!("消息负载 JSON 解析失败 [{}]: {}", subject, e)))?;

    info!(
        "收到消息: event_code={}, business_id={}, subject={}",
        message_mqo.event_code, message_mqo.business_id, subject
    );

    // ── 2. 查找消息模板 ──

    let cache = get_msg_cache();
    let msg: &MessageCache = cache.messages.get(&message_mqo.event_code).ok_or_else(|| {
        NatsError::Handle(format!(
            "未找到事件编码对应的消息模板: {}",
            message_mqo.event_code
        ))
    })?;

    // ── 3. 校验队列匹配 ──

    if msg.queue.code != subject {
        return Err(NatsError::Handle(format!(
            "消息队列不匹配: 模板队列={}, 实际队列={}",
            msg.queue.code, subject
        )));
    }

    // ── 4. 渲染标题和内容 ──
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
        return Err(NatsError::Handle("标题和内容均为空，无法投递".into()));
    }

    info!("内容渲染完成: title={title}");

    // ── 5. 幂等检查 ──

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

    // ── 6. 创建投递记录 ──

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
                .unwrap_or_else(|| now_ms)
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

    // ── 7. 创建投递目标 ──

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

                // ── 8. 直接用目标内嵌的渠道列表创建投递渠道记录 ──

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
