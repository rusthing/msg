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

use crate::cache::{ChannelCache, MessageCache, get_message_cache};
use crate::config::get_msg_config;
use crate::dto::MsgMessageQueueQueryDto;
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc, MsgMessageQueueSvc};
use crate::utils::render_template;
use crate::vo::MsgMessageQueueVo;
use crate::wkr::{self, channel::SendOutcome, deliver_result};
use arc_swap::ArcSwapOption;
use idworker::next_id;
use msg_api::cst::{MSG_MESSAGE_TOPIC, MSG_QUEUE_REFRESH_SUBJECT};
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::dto::{
    MsgDeliveryAddDto, MsgDeliveryChannelAddDto, MsgDeliveryQueryDto, MsgDeliveryTargetAddDto,
    MsgMessageQueueAddDto,
};
use msg_api::msg::{DeliveryMsg, message_msg::MessageMsg};
use robotech::api::U64;
use robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID;
use robotech::dao::DaoError;
use robotech::db::get_db_conn;
use robotech::mq::nats;
use robotech::mq::nats::NatsError;
use robotech::svc::SvcError;
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

/// # 保底注册消息入口队列
///
/// 所有发布方（msg-pub）都往 [`MSG_MESSAGE_TOPIC`]（`msg.message`）发消息，
/// msg-svr 不订阅这个 subject，整条链路就不通——既没有订阅，也没有 JetStream
/// stream，发布方的消息直接蒸发。
///
/// 队列表设计上由业务方注册，但**入口队列是 msg 自身的依赖**，不能指望外部
/// 先插好数据（`msg_message_queue` 目前没有任何写入方，见设计文档）。
/// 因此启动时直接新增一条 `persisted = true` 的记录：
/// JetStream 订阅会创建覆盖 `msg.message` 的 stream，发布方无论等不等 ACK
/// 都能被服务端落盘。
///
/// 重复注册不算失败：`code` 上有唯一约束，已存在时插入会返回
/// [`DaoError::DuplicateKey`]，与 HTTP 侧「RTW00001 唯一键重复」是同一个语义
/// （见 dbz 注册健康检查任务的写法），这里直接当作已完成。
///
/// 只在启动流程中调用一次，不参与运行期的队列刷新，避免刷新链路上多一次写库。
pub async fn ensure_intake_queue() -> anyhow::Result<()> {
    let db = get_db_conn()?;

    let add_result = MsgMessageQueueSvc::add(
        MsgMessageQueueAddDto {
            code: Some(MSG_MESSAGE_TOPIC.to_string()),
            name: Some("消息入口".to_string()),
            persisted: Some(true),
            remark: Some(Some("自动注册：msg-pub 统一发布入口".to_string())),
            _current_user_id: U64(SYS_OPERATOR_USER_ID),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await;

    match add_result {
        Ok(_) => {
            info!("消息入口队列注册成功: {MSG_MESSAGE_TOPIC}");
            Ok(())
        }
        Err(SvcError::Dao(DaoError::DuplicateKey(..))) => {
            info!("消息入口队列已存在: {MSG_MESSAGE_TOPIC}");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
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

/// NATS 资源名中 subject 片段的最大长度
///
/// Stream / Consumer 名会被写进 JetStream 的存储路径，官方建议控制在 32 字符
/// 以内（`MSG-` / `msg-svr-` 前缀另外占位），这里截断后仍留出哈希空间。
const MAX_NAME_TOKEN_LEN: usize = 24;

/// # 由 subject 派生 NATS 资源名片段
///
/// JetStream 的 stream / consumer 名禁止 `.` `*` `>` `/` 和空白，
/// 而队列 `code` 本身就是 NATS subject（例如 `msg.message`），直接拼进
/// `MSG-{code}` 会被服务端拒绝，订阅压根建不起来。
///
/// 规则：
/// - 非法字符统一替换成 `-`；超长时截断到 [`MAX_NAME_TOKEN_LEN`]；
/// - **发生过替换或截断时追加原值的短哈希**，避免 `a.b` 与 `a-b` 归一后撞名，
///   也避免截断后两个长 subject 撞名；
/// - 全是合法字符且不超长的 subject **原样保留**，已有 stream / consumer 不受影响。
fn nats_name_token(subject: &str) -> String {
    let mut token: String = subject
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();

    let lossy = token != subject || token.len() > MAX_NAME_TOKEN_LEN;
    if token.len() > MAX_NAME_TOKEN_LEN {
        token.truncate(MAX_NAME_TOKEN_LEN);
    }
    if lossy {
        token.push('-');
        token.push_str(&format!("{:08x}", fnv1a_32(subject.as_bytes())));
    }

    token
}

/// FNV-1a 32 位哈希，用于给派生名加去重后缀（不引入额外依赖）
fn fnv1a_32(bytes: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in bytes {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// # 为指定队列启动 NATS 订阅
///
/// - `persisted = false` → Core NATS 订阅（at-most-once）
/// - `persisted = true` → JetStream Push 订阅，创建的 Stream 名称为 `MSG-{token}`，
///   Consumer 名称为 `msg-svr-{token}`；`token` 由 [`nats_name_token`] 从 subject
///   派生（NATS 资源名禁止 `.` `*` `>` 等字符，不能直接拼 subject）
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
        // 队列 code 就是 NATS subject，不能直接拼进 JetStream 资源名（`.` 非法）
        let name_token = nats_name_token(subject);
        let stream_name = format!("MSG-{name_token}");
        let stream_config = stream::Config {
            name: stream_name.clone(),
            subjects: vec![subject.to_string()],
            // 保留 7 天：消费端长时间停机后仍能补上，同时避免 stream 无限增长
            // （`max_age` 默认 0 表示永久保留，文件存储会一直占盘）
            max_age: Duration::from_secs(7 * 24 * 3600),
            ..Default::default()
        };

        let consumer_config = consumer::push::Config {
            durable_name: Some(format!("msg-svr-{name_token}")),
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
/// 4. 渲染标题和内容（模板变量取 `labels` 与 `annotations`；模板为 null 时从 `annotations` 取）
/// 5. 幂等检查（`(event_code, business_id, business_trigger_ms)` 唯一约束；
///    非持久化消息在第 4 步后就返回，不走到这里）
/// 6. 创建 `msg_delivery` 投递记录（含 labels / annotations 快照，供排查）
/// 7. 为每个目标创建 `msg_delivery_target`
/// 8. 为每个（目标 × 渠道）组合创建 `msg_delivery_channel`
///
/// 投递任务带上完整的一条消息：渲染好的标题正文、原始 `labels` / `annotations`、
/// 以及消息类别与来源的编码名称。模板只把变量拼成文案，不吞掉输入——接收方
/// 还要用结构化字段做业务处理（大屏按坐标落点、按楼层切图）。
///
/// ## 返回值
///
/// - `Ok(())`：消息处理成功（含幂等跳过）
/// - `Err(NatsError)`：处理失败；持久化消息缺 `business_id` / `business_trigger_ms`
///   时是 `Discard`（不重投，重复投递只会堆垃圾记录）
///
/// JetStream 模式下 `Ok(())` 会触发 ACK，`Err` 会导致消息被重新投递；
/// 只有 `Discard` 是终止性的，直接 ACK 丢弃。
pub async fn process_message(subject: &str, payload: &[u8]) -> Result<(), NatsError> {
    // ── 1. 解析消息负载 ──

    let message_msg: MessageMsg = serde_json::from_slice(payload)
        .map_err(|e| NatsError::Discard(format!("消息负载 JSON 解析失败 [{}]: {}", subject, e)))?;

    info!(
        "收到消息: event_code={}, business_id={:?}, subject={}",
        message_msg.event_code, message_msg.business_id, subject
    );

    // ── 2. 查找消息模板 ──

    let cache = get_message_cache();
    let msg: &MessageCache = cache.get(&message_msg.event_code).ok_or_else(|| {
        NatsError::Handle(format!(
            "未找到事件编码对应的消息模板: {}",
            message_msg.event_code
        ))
    })?;

    // ── 3. 渲染标题和内容 ──
    //
    //   模板只负责把 labels 与 annotations 拼成可读文案，渲染结果和原始变量
    //   一起下发：接收方既要文案，也要结构化字段。
    //
    //   title_template 不为 null → 用 labels + annotations 做变量替换
    //   title_template 为 null    → 取 annotations["title"]
    //
    //   content_template 不为 null → 用 labels + annotations 做变量替换
    //   content_template 为 null    → 取 annotations["content"]

    let title = match &msg.title_template {
        Some(tpl) => render_template(tpl, &message_msg.labels, &message_msg.annotations),
        None => message_msg
            .annotations
            .get("title")
            .cloned()
            .unwrap_or_default(),
    };

    let content = match &msg.content_template {
        Some(tpl) => render_template(tpl, &message_msg.labels, &message_msg.annotations),
        None => message_msg
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

    let now_ms = chrono::Utc::now().timestamp_millis() as u64;

    // ── 4. 非持久化消息：不落库、不回写，发不出去就丢 ──
    //
    //   `msg_message.persisted = false` 的消息没有投递记录（三级 ID 全空），
    //   也没有幂等可言（没地方存），所以直接按缓存里的目标×渠道拆任务发出去，
    //   失败只打日志。这类消息的语义就是 at-most-once。

    if !msg.persisted {
        publish_best_effort(msg, &message_msg, &title, &content, now_ms).await;
        return Ok(());
    }

    // ── 5. 幂等检查 ──
    //
    //   幂等键与 `msg_delivery` 的唯一约束保持一致：
    //   `(event_code, business_id, business_trigger_ms)`。
    //   只按 business_id 查会把「同一个业务对象的另一个事件」误判成重复，
    //   直接静默漏投。
    //
    //   两个业务字段在 `MessageMsg` 里都是可选的：发布方没有业务对象可指时会留空
    //   （不落库的消息填不填都不参与幂等，见下面的分支）。落库的消息必须有，
    //   缺了就地拒收——
    //   不落库就没地方存幂等键，硬投出去只会让同一条消息反复生成投递记录。
    //   服务端也不兜底：补 now 会让每次重投都算新消息，补 0 又会吞掉后续真实触发。

    let (Some(business_id), Some(business_trigger_ms)) = (
        message_msg.business_id,
        message_msg.business_trigger_ms,
    ) else {
        return Err(NatsError::Discard(format!(
            "持久化消息缺少幂等键 [{}]: business_id={:?}, business_trigger_ms={:?}",
            subject, message_msg.business_id, message_msg.business_trigger_ms
        )));
    };

    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;

    let existing = MsgDeliverySvc::get_by_query_dto(
        MsgDeliveryQueryDto {
            event_code: Some(message_msg.event_code.clone()),
            business_id: Some(business_id.into()),
            business_trigger_ms: Some(business_trigger_ms.into()),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await;

    match existing {
        Ok(ro) if ro.extra.is_some() => {
            info!(
                "消息已投递，幂等跳过: event_code={}, business_id={business_id}, business_trigger_ms={business_trigger_ms}",
                message_msg.event_code
            );
            return Ok(());
        }
        Ok(_) => {}
        Err(e) => {
            warn!("幂等检查查询失败，继续处理: {e}");
        }
    }

    // ── 6. 创建投递记录 ──

    let delivery_id = next_id().map_err(|e| NatsError::Handle(format!("生成投递ID失败: {e}")))?;

    let labels_json = if message_msg.labels.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&message_msg.labels)
                .map_err(|e| NatsError::Handle(format!("labels 序列化失败: {e}")))?,
        )
    };

    let annotations_json = if message_msg.annotations.is_empty() {
        None
    } else {
        Some(
            serde_json::to_string(&message_msg.annotations)
                .map_err(|e| NatsError::Handle(format!("annotations 序列化失败: {e}")))?,
        )
    };

    let delivery_add = MsgDeliveryAddDto {
        id: Some(U64(delivery_id)),
        event_code: Some(msg.event_code.clone()),
        message_id: Some(msg.id.into()),
        event_source_id: Some(msg.source.id.into()),
        message_category_id: Some(msg.category.id.into()),
        business_id: Some(business_id.into()),
        // 与幂等检查用的是同一个值，不能再各自兜底
        business_trigger_ms: Some(business_trigger_ms.into()),
        deliver_status: Some(DeliverStatus::Delivering),
        labels: Some(labels_json),
        annotations: Some(annotations_json),
        // 后面还要用它们组装投递任务（任务里带的是渲染好的文案）
        title: Some(title.clone()),
        content: Some(content.clone()),
        remark: None,
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
    };

    MsgDeliverySvc::add(delivery_add, Some(db.as_ref()))
        .await
        .map_err(|e| {
            NatsError::Handle(format!(
                "创建投递记录失败 (business_id={business_id}): {e}"
            ))
        })?;

    info!(
        "投递记录已创建: delivery_id={delivery_id}, message_id={}",
        msg.id
    );

    // ── 7. 创建投递目标与渠道记录，并把投递任务收起来 ──

    // 任务要等记录建好再发：先有 ID 才能回写，发不出去也能在库里看到
    let task_ctx = TaskContext {
        msg,
        message_msg: &message_msg,
        title: &title,
        content: &content,
        created_ms: now_ms,
    };
    let mut tasks: Vec<DeliveryMsg> = Vec::new();

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
                        tasks.push(build_task(
                            &task_ctx,
                            (
                                Some(delivery_id),
                                Some(created_target.id.0),
                                Some(channel_id_val),
                            ),
                            target.target_id,
                            channel,
                        ));
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

    // ── 9. 发布投递任务 ──

    for task in tasks {
        if let Err(e) = wkr::publish_task(&task).await {
            error!(
                "发布投递任务失败: channel={}, target_id={}, error={e}",
                task.channel_code, task.target_id
            );
            // 发不出去就不会有发送结果回来，就地记失败，避免渠道一直挂在「投递中」
            deliver_result::record(
                &task,
                &SendOutcome::Failed(format!("发布投递任务失败: {e}")),
            )
            .await;
        }
    }

    info!(
        "消息处理完成: event_code={}, business_id={:?}",
        message_msg.event_code, message_msg.business_id
    );

    Ok(())
}

/// # 组装投递任务时共用的上下文
///
/// 持久化与非持久化两条路径只有「投递 ID 有没有」不同，其余都取自这里。
struct TaskContext<'a> {
    msg: &'a MessageCache,
    message_msg: &'a MessageMsg,
    title: &'a str,
    content: &'a str,
    created_ms: u64,
}

/// # 组装一条投递任务
///
/// `ids` 是 `(delivery_id, delivery_target_id, delivery_channel_id)`：
/// 持久化消息带真实 ID，非持久化消息全为 `None`。
fn build_task(
    ctx: &TaskContext<'_>,
    ids: (Option<u64>, Option<u64>, Option<u64>),
    target_id: u64,
    channel: &ChannelCache,
) -> DeliveryMsg {
    DeliveryMsg {
        persisted: ctx.msg.persisted,
        delivery_id: ids.0,
        delivery_target_id: ids.1,
        delivery_channel_id: ids.2,
        event_code: ctx.msg.event_code.clone(),
        message_id: ctx.msg.id,
        message_name: ctx.msg.name.clone(),
        category_code: ctx.msg.category.code.clone(),
        category_name: ctx.msg.category.name.clone(),
        source_code: ctx.msg.source.code.clone(),
        source_name: ctx.msg.source.name.clone(),
        channel_id: channel.id,
        channel_code: channel.code.clone(),
        target_id,
        business_id: ctx.message_msg.business_id,
        business_trigger_ms: ctx.message_msg.business_trigger_ms,
        // 原始变量原样带下去：模板渲染出的标题正文只是其中一种用法
        labels: ctx.message_msg.labels.clone(),
        annotations: ctx.message_msg.annotations.clone(),
        title: ctx.title.to_string(),
        content: ctx.content.to_string(),
        created_ms: ctx.created_ms,
    }
}

/// # 非持久化消息的投递
///
/// 不写任何表、不做幂等、不回写状态：按缓存里的目标×渠道直接发，
/// 发不出去只打日志。这类消息语义就是 at-most-once。
async fn publish_best_effort(
    msg: &MessageCache,
    message_msg: &MessageMsg,
    title: &str,
    content: &str,
    created_ms: u64,
) {
    let ctx = TaskContext {
        msg,
        message_msg,
        title,
        content,
        created_ms,
    };

    for target in &msg.targets {
        for channel in &target.target_category.channels {
            let task = build_task(&ctx, (None, None, None), target.target_id, channel);
            if let Err(e) = wkr::publish_task(&task).await {
                warn!(
                    "发布非持久化投递任务失败，已丢弃: channel={}, target_id={}, error={e}",
                    task.channel_code, task.target_id
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// NATS stream / consumer 名允许的字符集
    fn is_valid_nats_name(name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }

    #[test]
    fn safe_subject_is_kept_as_is() {
        // 纯字母数字的名字不变，避免部署后已存在的 stream / consumer 被改名
        assert_eq!(nats_name_token("msg"), "msg");
        assert_eq!(nats_name_token("order_created"), "order_created");
        assert_eq!(nats_name_token("order-created"), "order-created");
    }

    #[test]
    fn dotted_subject_is_sanitized() {
        let token = nats_name_token("msg.message");
        assert!(is_valid_nats_name(&token), "仍含非法字符: {token}");
        assert!(token.starts_with("msg-message-"), "token={token}");
    }

    #[test]
    fn sanitized_names_do_not_collide() {
        // `a.b` 与 `a-b` 归一后必须仍然不同，否则两个队列会共用一个 stream
        assert_ne!(nats_name_token("a.b"), nats_name_token("a-b"));
    }

    #[test]
    fn long_subject_is_truncated_and_still_valid() {
        let subject = "a".repeat(60);
        let token = nats_name_token(&subject);
        assert!(is_valid_nats_name(&token));
        assert!(token.len() <= MAX_NAME_TOKEN_LEN + 9, "token={token}");
        assert_ne!(token, nats_name_token(&"b".repeat(60)));
    }
}
