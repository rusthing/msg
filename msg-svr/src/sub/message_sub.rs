//! # 消息队列订阅管理
//!
//! 维护当前活跃的消息队列订阅句柄，队列变更时根据最新的队列信息同步订阅
//! 状态：新增队列则启动新订阅，移除的队列则停止旧的订阅线程。
//!
//! ## 数据流
//!
//! ```text
//! 队列变更信号（启动加载 / 队列刷新信号）
//!   → 从 DB 查询所有 msg_message_queue
//!   → 映射为 MessageQueueSpec（订阅只需要 code + persisted）
//!   → sync_message_queue_subscriptions(specs)
//!     → 对比旧订阅：新增 / 保留 / 移除
//!     → 调用 NATS subscribe 启动新线程
//!     → 停止不再需要的订阅线程
//! ```
//!
//! 队列信息**不进缓存**：它唯一的用途是决定订阅哪些 subject，
//! 而订阅状态本身由 [`MESSAGE_QUEUE_SUBSCRIBERS`] 维护。缓存一份队列数据只会
//! 多一份需要同步的快照，没有任何读取方。
//!
//! 注意：消息模板缓存（`build_message_cache`）不会触发队列重新订阅。

use super::target_expand::{ExpandOutcome, ExpandedChannel, ExpandedTarget, expand_targets};
use crate::cache::{MessageCache, get_message_cache};
use crate::config::get_msg_config;
use crate::dto::MsgMessageQueueQueryDto;
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc, MsgMessageQueueSvc};
use crate::utils::{render_template, server_now_ms};
use crate::vo::MsgMessageQueueVo;
use crate::wkr::{self, channel::SendOutcome, deliver_result};
use arc_swap::ArcSwapOption;
use idworker::next_id;
use msg_api::cst::{MSG_MESSAGE_NORMAL_TOPIC, MSG_MESSAGE_QUEUE_REFRESH_SUBJECT};
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::dto::{
    MsgDeliveryAddDto, MsgDeliveryChannelAddDto, MsgDeliveryChannelQueryDto, MsgDeliveryModifyDto,
    MsgDeliveryQueryDto, MsgDeliveryTargetAddDto, MsgDeliveryTargetModifyDto,
    MsgDeliveryTargetQueryDto, MsgMessageQueueAddDto,
};
use msg_api::msg::{DeliveryMsg, message_msg::MessageMsg};
use robotech::api::U64;
use robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID;
use robotech::dao::DaoError;
use robotech::db::get_db_conn;
use robotech::mq::nats;
use robotech::mq::nats::NatsError;
use robotech::svc::SvcError;
use sea_orm::{ConnectionTrait, DatabaseConnection, TransactionTrait};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// `msg_delivery` 的表名与幂等键列
///
/// 幂等键 `(event_code, business_id, business_trigger_ms)` 就是这个唯一约束。
/// 处理流程不再先查一遍再插，直接靠它兜住重复投递（见 `process_message` 第
/// 5+6 步），所以这里要把「哪张表的哪个键」写死成常量做精确匹配——按下标或
/// 模糊匹配会把别处的重复键误判成「已经投递过」，那是静默漏投。
const DELIVERY_TABLE: &str = "msg_delivery";
const DELIVERY_IDEM_KEY: &str = "event_code,business_id,business_trigger_ms";

/// # 消息队列订阅规格
///
/// 订阅层只关心「订阅哪个 subject、是否持久化」，
/// 不需要完整的队列实体。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageQueueSpec {
    /// 队列编码，同时作为 NATS subject
    pub code: String,
    /// 是否持久化（`true` → JetStream，`false` → Core NATS）
    pub persisted: bool,
}

/// # 消息队列订阅句柄
///
/// 除了订阅任务句柄，还记录「建立订阅时应用的重投上限」：配置热更新改了
/// `msg.max-deliver` 之后，靠这个值判断哪些 consumer 需要重建。
#[derive(Debug)]
struct MessageQueueSubscription {
    /// 订阅任务句柄
    handle: Arc<JoinHandle<()>>,
    /// 是否 JetStream 订阅（Core NATS 没有 consumer，重投上限对它无意义）
    persisted: bool,
    /// 建立订阅时应用的重投上限
    max_deliver: i64,
}

/// # 全局消息队列订阅句柄
///
/// key = `queue.code`（即 NATS subject），value = 订阅线程句柄。
/// 使用 [`ArcSwapOption`] 实现无锁热替换。
static MESSAGE_QUEUE_SUBSCRIBERS: ArcSwapOption<HashMap<String, MessageQueueSubscription>> =
    ArcSwapOption::const_empty();

/// # 重新加载消息队列订阅
///
/// 从 DB 查询全量消息队列并同步订阅。启动时调用一次，之后每次收到队列刷新
/// 信号都调用——这是队列变更后唯一的处理入口，没有缓存快照需要维护。
pub async fn reload_message_queue_subscriptions() -> anyhow::Result<()> {
    info!("消息队列订阅同步...");

    let db = get_db_conn()?;
    let queues: Vec<MsgMessageQueueVo> = MsgMessageQueueSvc::list_by_query_dto(
        MsgMessageQueueQueryDto::default(),
        Some(db.as_ref()),
    )
    .await?
    .extra
    .unwrap_or_default();

    let specs: HashSet<MessageQueueSpec> = queues
        .into_iter()
        .map(|q| MessageQueueSpec {
            code: q.code,
            persisted: q.persisted,
        })
        .collect();

    sync_message_queue_subscriptions(&specs).await;
    info!("消息队列订阅同步完成");
    Ok(())
}

/// # 保底注册普通消息队列
///
/// 业务方不单独声明消息队列时，发布方都往 [`MSG_MESSAGE_NORMAL_TOPIC`]
/// （`msg.message.normal`）发消息，msg-svr 不订阅这个 subject，整条链路就不通
/// ——既没有订阅，也没有 JetStream stream，发布方的消息直接蒸发。
///
/// 队列表设计上由业务方注册，但**普通消息队列是 msg 自身的依赖**，不能指望
/// 外部先插好数据。因此启动时直接新增一条 `persisted = true` 的记录：
/// JetStream 订阅会创建覆盖 `msg.message.normal` 的 stream，发布方无论等不等
/// ACK 都能被服务端落盘。
///
/// 重复注册只有 **`code` 冲突**才算「已注册」：`code` 就是订阅与发布共用的
/// subject，`code = msg.message.normal` 的行在，默认入口才真的存在。`name` 上
/// 也有唯一约束，但它是给运营看的展示名，别的队列先占了「普通消息队列」这个名字
/// 时，那条队列的 subject 不是 `msg.message.normal`，msg-svr 依然没有默认入口
/// ——把这种冲突也当成成功，发布方会一直报「no stream matches subject」，而启动
/// 日志里看不出任何异常。所以 `name` 冲突必须打明确错误并让启动失败。
///
/// 既有行还要校验 `persisted`：队列存在但 `persisted = false` 时订阅走 Core
/// NATS，没有 JetStream stream，而 `msg-pub` 的默认发布会等 ACK，结果是发布
/// 直接失败。这不是能自动纠正的状态（改配置属于运营决策），同样让启动失败。
///
/// 只在启动流程中调用一次，不参与运行期的队列刷新，避免刷新链路上多一次写库。
pub async fn ensure_normal_message_queue() -> anyhow::Result<()> {
    let db = get_db_conn()?;

    let add_result = MsgMessageQueueSvc::add(
        MsgMessageQueueAddDto {
            code: Some(MSG_MESSAGE_NORMAL_TOPIC.to_string()),
            name: Some(MSG_MESSAGE_NORMAL_QUEUE_NAME.to_string()),
            persisted: Some(true),
            remark: Some(Some("msg-pub 默认发布入口".to_string())),
            _current_user_id: U64(SYS_OPERATOR_USER_ID),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await;

    match add_result {
        Ok(_) => {
            info!("普通消息队列注册成功: {MSG_MESSAGE_NORMAL_TOPIC}");
            Ok(())
        }
        Err(SvcError::Dao(DaoError::DuplicateKey(key, value))) => match key.key_name.as_str() {
            "code" => ensure_existing_normal_queue(db.as_ref()).await,
            "name" => {
                let conflict = MsgMessageQueueSvc::get_by_query_dto(
                    MsgMessageQueueQueryDto {
                        name: Some(MSG_MESSAGE_NORMAL_QUEUE_NAME.to_string()),
                        ..Default::default()
                    },
                    Some(db.as_ref()),
                )
                .await?
                .extra;

                let detail = conflict
                    .map(|q| {
                        format!(
                            "id={}, code(subject)={}, name={}, persisted={}",
                            q.id, q.code, q.name, q.persisted
                        )
                    })
                    .unwrap_or_else(|| "按 name 回查不到冲突行".to_string());

                error!(
                    "普通消息队列注册失败：name={MSG_MESSAGE_NORMAL_QUEUE_NAME:?} 已被占用，\
                     期望 code(subject)={MSG_MESSAGE_NORMAL_TOPIC}；冲突行: {detail}。\
                     默认发布入口不存在，msg-pub 发布普通消息会失败，启动中止"
                );
                Err(anyhow::anyhow!(
                    "普通消息队列 name={MSG_MESSAGE_NORMAL_QUEUE_NAME:?} 与既有队列冲突（冲突值={value}），\
                     无法注册 {MSG_MESSAGE_NORMAL_TOPIC}；请先修正冲突行的 name"
                ))
            }
            other => Err(anyhow::anyhow!(
                "普通消息队列注册失败：未预期的唯一键冲突 {other}={value}，无法确认 {MSG_MESSAGE_NORMAL_TOPIC} 是否已注册"
            )),
        },
        Err(e) => Err(e.into()),
    }
}

/// 普通消息队列的 `name`
///
/// `name` 与 `code` 是两条独立的唯一键，冲突时插入报的键名不同，处理方式也
/// 不同（见 [`ensure_normal_message_queue`]）。
const MSG_MESSAGE_NORMAL_QUEUE_NAME: &str = "普通消息队列";

/// # 校验已存在的普通消息队列
///
/// `code` 冲突说明 subject 已经被占用，但「存在」不等于「可用」：
/// `persisted = false` 时没有 JetStream stream，默认持久化发布会失败。
/// 这里只报告事实并让启动失败，不替运营改配置。
async fn ensure_existing_normal_queue(db: &DatabaseConnection) -> anyhow::Result<()> {
    let existing = MsgMessageQueueSvc::get_by_query_dto(
        MsgMessageQueueQueryDto {
            code: Some(MSG_MESSAGE_NORMAL_TOPIC.to_string()),
            ..Default::default()
        },
        Some(db),
    )
    .await?
    .extra;

    match existing {
        Some(queue) if queue.persisted => {
            info!(
                "普通消息队列已存在: code={}, name={}, persisted=true",
                queue.code, queue.name
            );
            Ok(())
        }
        Some(queue) => {
            error!(
                "普通消息队列已存在但 persisted=false: code={}, name={}。\
                 该队列走 Core NATS，不会创建 JetStream stream，msg-pub 的默认持久化发布会失败，启动中止",
                queue.code, queue.name
            );
            Err(anyhow::anyhow!(
                "普通消息队列 {} 的 persisted=false，默认持久化发布会失败；请将其改为 true",
                queue.code
            ))
        }
        None => Err(anyhow::anyhow!(
            "普通消息队列注册报 code 冲突，但回查不到 code={MSG_MESSAGE_NORMAL_TOPIC} 的记录"
        )),
    }
}

/// # 消息队列刷新信号
///
/// 用于 `#[svc(after_write = ...)]`：队列配置写成功后通知所有节点同步订阅。
pub async fn on_message_queue_refresh() {
    if let Err(e) = robotech::refresh::publish(&MSG_MESSAGE_QUEUE_REFRESH_SUBJECT).await {
        error!("消息队列刷新信号失败: {e}");
    }
}

/// # 同步消息队列订阅
///
/// 根据给定的队列集合，对比当前活跃订阅：
/// - 队列 `code` 不存在于当前订阅 → 启动新订阅
/// - 队列 `code` 已存在且订阅任务仍在运行 → 保留（跳过）
/// - 队列 `code` 已存在但订阅任务**已退出** → 视为不存在，重建
/// - 队列 `code` 已存在但**持久化方式已变更** → 停掉旧订阅，按新方式重建
/// - 队列 `code` 已存在但**重投上限已变更** → 停掉旧订阅，按新上限重建
/// - 当前订阅中的 `code` 不在 `queues` 中 → 停止
///
/// 「任务已退出」必须算作不存在：连接关闭（如 NATS 配置热更新换掉 Client）
/// 会让订阅任务自行结束，但句柄仍留在 [`MESSAGE_QUEUE_SUBSCRIBERS`] 里。只比 `code`
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
pub async fn sync_message_queue_subscriptions(queues: &HashSet<MessageQueueSpec>) {
    let current = MESSAGE_QUEUE_SUBSCRIBERS
        .load_full()
        .unwrap_or_else(|| Arc::new(HashMap::new()));

    let new_codes: HashSet<String> = queues.iter().map(|q| q.code.clone()).collect();

    // 重投上限来自配置文件，改配置后热更新会重新走到这里
    let max_deliver = get_msg_config().effective_max_deliver();

    // 队列当前的持久化方式：它决定订阅走 JetStream 还是 Core NATS，不是能原地
    // 刷新到 consumer 上的参数，只能重建订阅。
    let persisted_by_code: HashMap<&str, bool> = queues
        .iter()
        .map(|queue| (queue.code.as_str(), queue.persisted))
        .collect();

    // ── 保留仍然存活的订阅，死句柄/配置已变的直接丢弃（下面会按新增重建）──

    let mut next: HashMap<String, MessageQueueSubscription> = HashMap::new();
    let mut live_codes: HashSet<String> = HashSet::new();

    for (code, subscription) in current.iter() {
        if subscription.handle.is_finished() {
            warn!("队列订阅任务已退出，将重建: {code}");
            continue;
        }
        // 持久化方式变了必须重建：旧订阅还按老方式收，而发布方已经按新配置发。
        // 尤其是 false → true：没有 stream 时发布方的 JetStream 发布（默认模式）
        // 会一直报「no stream matches subject」，直到服务重启才恢复。
        if let Some(&persisted) = persisted_by_code.get(code.as_str())
            && persisted != subscription.persisted
        {
            info!(
                "队列订阅方式变更（persisted {} → {persisted}），重建订阅: {code}",
                subscription.persisted
            );
            subscription.handle.abort();
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
            MessageQueueSubscription {
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
            MessageQueueSubscription {
                handle,
                persisted,
                max_deliver,
            },
        );
    }

    MESSAGE_QUEUE_SUBSCRIBERS.store(Some(Arc::new(next)));
}

/// # 队列组名
///
/// 同一队列的所有 msg-svr 副本必须共用同一个组名，否则同一条消息会被
/// 每个副本各收一份（普通订阅语义是「所有订阅者都收到」）。
///
/// - JetStream 模式：写入 consumer 的 `deliver_group`
/// - Core NATS 模式：作为 `queue_subscribe` 的组名
///
/// 投递侧（[`crate::wkr::sub`]）的 JetStream 队列也用这个名字，两边靠
/// Stream / Consumer 名区分，不冲突。
pub(crate) const QUEUE_GROUP: &str = "msg-svr";

/// NATS 资源名中 subject 片段的最大长度
///
/// Stream / Consumer 名会被写进 JetStream 的存储路径，官方建议控制在 32 字符
/// 以内（`MSG-` / `msg-svr-` 前缀另外占位），这里截断后仍留出哈希空间。
const MAX_NAME_TOKEN_LEN: usize = 24;

/// # 由 subject 派生 NATS 资源名片段
///
/// JetStream 的 stream / consumer 名禁止 `.` `*` `>` `/` 和空白，
/// 而队列 `code` 本身就是 NATS subject（例如 `msg.message.normal`），直接拼进
/// `MSG-{code}` 会被服务端拒绝，订阅压根建不起来。
///
/// 规则：
/// - 非法字符统一替换成 `-`；超长时截断到 [`MAX_NAME_TOKEN_LEN`]；
/// - **发生过替换或截断时追加原值的短哈希**，避免 `a.b` 与 `a-b` 归一后撞名，
///   也避免截断后两个长 subject 撞名；
/// - 全是合法字符且不超长的 subject **原样保留**，已有 stream / consumer 不受影响。
///
/// 投递侧（[`crate::wkr::sub`]）也用它派生投递队列的 Stream / Consumer 名；
/// 两边 subject 不同（`msg.message.normal` vs `msg.delivery.websocket`），
/// 派生结果不会撞。
pub(crate) fn nats_name_token(subject: &str) -> String {
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
/// 3. 渲染标题、内容与附加内容（模板优先、发布方兜底；标题和内容为空就拒收）
/// 4. 非持久化消息直接按缓存里的目标×渠道投递并返回
/// 5. 幂等检查（`(event_code, business_id, business_trigger_ms)` 唯一约束）
/// 6. 同一事务内创建 `msg_delivery` 投递记录（含 labels / annotations 快照）、
///    每个目标的 `msg_delivery_target`、每个（目标 × 渠道）的 `msg_delivery_channel`
/// 7. 事务提交后再发布投递任务：消费者回写状态时要能查到渠道行
/// 8. 撞幂等键时检查既有投递：缺目标/渠道就补建并补发，历史半截数据自愈
/// 9. 一个任务都没有（没挂目标 / 目标类别没绑渠道）时报错并把投递记录收口成
///    「全部失败」
///
/// 投递任务与投递三张表一一对应：渲染好的标题、内容、附加内容，三张表的行 ID
/// 与目标/渠道字段。`labels` / `annotations` 只是模板的输入，渲染完就只剩
/// `msg_delivery` 里的快照，不再下发——接收方要的结构化字段，得由模板渲染
/// 进 `content` 或 `ext`。
///
/// ## 返回值
///
/// - `Ok(())`：消息处理成功（含幂等跳过）
/// - `Err(NatsError)`：处理失败；标题/内容为空、持久化消息缺 `business_id` /
///   `business_trigger_ms` 时是 `Discard`（配置错误，重投多少次都一样）
///
/// 带 `simulated=true` 标记的消息在模板查到时先过一道门控：
/// `msg_message.sim_enabled = false` 时直接丢弃（`Ok(())`，ACK 掉，不重投），
/// 不建投递记录、不发渠道、不上大屏。真实通知不带标记，不受影响；放行的
/// 模拟通知会在 `ext` 里带上 `simulated=true` 再投递（见第 3 步）。
///
/// JetStream 模式下 `Ok(())` 会触发 ACK，`Err` 会导致消息被重新投递；
/// 只有 `Discard` 是终止性的，直接 ACK 丢弃。
pub async fn process_message(subject: &str, payload: &[u8]) -> Result<(), NatsError> {
    let started = std::time::Instant::now();

    // ── 1. 解析消息负载 ──

    let message_msg: MessageMsg = serde_json::from_slice(payload)
        .map_err(|e| NatsError::Discard(format!("消息负载 JSON 解析失败 [{}]: {}", subject, e)))?;

    info!(
        "收到消息: event_code={}, business_id={:?}, subject={}",
        message_msg.event_code, message_msg.business_id, subject
    );

    // 与 dtw / alarm 侧共用的追踪键：
    // - `trigger_ms` 这里打的是 `business_trigger_ms`，也就是**这一次通知**的时间，
    //   每次通知都不同（同一条报警的多轮提醒靠它区分）。成立通知里它恰好等于上报
    //   进来的 `dto.trigger_ms`，确认 / 结束 / 重复提醒里则不是；
    // - 真正的事件起点是 `MessageMsg.trigger_ms`（报警侧给 `first_trigger_ms`），
    //   不参与幂等，落在 `msg_delivery.trigger_ms` 上用来算端到端延迟，可能为空；
    // - `bid` 在这一跳是**报警 ID**（发布方填的 `business_id`），dtw/alarm 那里
    //   是卡片编码的哈希。两侧对得上靠 alarm 日志里同时打出的 `alert_id`。
    let trace = format!(
        "sos_trace bid={:?} trigger_ms={:?} hop=msg.process subject={}",
        message_msg.business_id, message_msg.business_trigger_ms, subject
    );

    // ── 2. 查找消息模板 ──

    let cache = get_message_cache();
    // 模板编码与事件编码默认一致；发布方显式给了 `template_code` 时按它查模板。
    let template_code = message_msg
        .template_code
        .as_deref()
        .unwrap_or(&message_msg.event_code);
    let msg: &MessageCache = cache.get(template_code).ok_or_else(|| {
        NatsError::Handle(format!("未找到事件编码对应的消息模板: {}", template_code))
    })?;

    // ── 2.5 模拟通知门控 ──
    //
    // 模拟产生的通知只在 `msg_message.sim_enabled` 打开时才允许投递。关闭时
    // 就地丢弃并 ACK：继续往下走会落投递记录、推渠道（大屏），而模拟数据不该
    // 出现在生产屏上。真实通知不带 `simulated` 标记，不受影响。
    if is_simulated(&message_msg) && !msg.sim_enabled {
        warn!(
            "模拟通知被丢弃: message_id={}, event_code={}, business_id={:?}, 原因=消息未开启「允许模拟投递」（带 simulated=true 标记）",
            msg.id, msg.event_code, message_msg.business_id
        );
        return Ok(());
    }

    // ── 3. 渲染标题、内容与附加内容 ──
    //
    //   三个字段各自独立判断，模板优先、发布方兜底：
    //   msg_message 上的模板有值 → 用模板渲染；
    //   模板为 null → 取 MessageMsg 里发布方给的同名值；
    //   两边都没有，或渲染结果全是空白 → None（空串不算有内容）。
    //
    //   模板变量取 labels / annotations，这两份只是输入，不随投递下发。
    //   模板为 null 时只认发布方给的字段，不会再去 annotations 里找同名键
    //   兜底（早先 title 缺模板会去 annotations 取 title，现在不发 annotations
    //   了，那条路已经没有了）。
    //
    //   除发布方给的变量外，服务端再注入一批 `_` 开头的内置变量（处理时间、
    //   业务触发时间等），发布方不必为了打时间戳而自己往 labels 里塞。

    // 后面落库和投递都用这个时间，提前到这里是为了让模板能引用 `_create_ms`
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let builtins = template_builtins(msg, &message_msg, now_ms);

    let title = pick(
        msg.title_template.as_deref(),
        message_msg.title.as_deref(),
        &builtins,
        &message_msg.labels,
        &message_msg.annotations,
    );

    let content = pick(
        msg.content_template.as_deref(),
        message_msg.content.as_deref(),
        &builtins,
        &message_msg.labels,
        &message_msg.annotations,
    );

    let ext = pick(
        msg.ext_template.as_deref(),
        message_msg.ext.as_deref(),
        &builtins,
        &message_msg.labels,
        &message_msg.annotations,
    );

    // 让模拟标记随 ext 一起上大屏。labels / annotations 只是模板输入，不随投递
    // 下发；而 ext 在三处各自生成（报警策略模板、位置消息定义模板、恢复消息
    // 固定 JSON），靠每条模板都记得写 `{{simulated:json}}` 必然漏配，存量库里的
    // 旧模板更不会被重写。在渲染收口之后统一补一次，三类消息一网打尽。
    //
    // 只有过了 2.5 门控（msg_message.sim_enabled 已打开）的模拟消息才会走到
    // 这里；真实消息完全不进这个分支，ext 一位不改。
    let ext = if is_simulated(&message_msg) {
        inject_simulated_ext(ext.as_deref())
    } else {
        ext
    };

    info!("内容渲染完成: title={title:?}, content={content:?}, ext={ext:?}");

    // 标题和内容是投递的成品，缺一个消费方就没法用。这是配置错误
    // （模板和发布方都没给），重投多少次结果都一样，就地拒收。
    let (title, content) = match (title, content) {
        (Some(title), Some(content)) => (title, content),
        (title, content) => {
            return Err(NatsError::Discard(format!(
                "标题或内容为空，拒收 [{}]: event_code={}, title={title:?}, content={content:?}",
                subject, message_msg.event_code
            )));
        }
    };

    // `msg_delivery.title` 是 varchar(150)，标题模板本身就能顶满 150，再叠加
    // 变量（或发布方直传）很容易超长；超长会在建投递记录时校验/入库失败，消息
    // 重投几次后进死信，一条都投不出去。标题只用于展示，正文里已有完整内容，
    // 这里按字符（不是字节，按字节截中文会截出坏字符）截断并留一条 warn。
    let title = truncate_title(title);

    // ── 4. 非持久化消息：不落库、不回写，发不出去就丢 ──
    //
    //   `msg_message.persisted = false` 的消息没有投递记录（三张表的行 ID 全空），
    //   也没有幂等可言（没地方存），所以直接按缓存里的目标×渠道拆任务发出去，
    //   失败只打日志。这类消息的语义就是 at-most-once。

    if !msg.persisted {
        let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;
        let expanded = expand_targets(db.as_ref(), msg, &message_msg.labels)
            .await
            .map_err(|e| NatsError::Handle(format!("展开投递目标失败: {e}")))?;
        publish_best_effort(
            msg,
            &message_msg,
            &title,
            &content,
            ext.as_deref(),
            now_ms,
            &expanded,
        )
        .await;
        info!(
            "{trace} cost_ms={} result=best_effort",
            started.elapsed().as_millis()
        );
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

    let (Some(business_id), Some(business_trigger_ms)) =
        (message_msg.business_id, message_msg.business_trigger_ms)
    else {
        return Err(NatsError::Discard(format!(
            "持久化消息缺少幂等键 [{}]: business_id={:?}, business_trigger_ms={:?}",
            subject, message_msg.business_id, message_msg.business_trigger_ms
        )));
    };

    // 任务要等记录建好再发：先有 ID 才能回写，发不出去也能在库里看到
    let task_ctx = TaskContext {
        msg,
        message_msg: &message_msg,
        title: &title,
        content: &content,
        ext: ext.as_deref(),
        create_ms: now_ms,
    };

    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;

    // 抽象目标 → 人 + 地址：只读查询放事务外，摊到事务里只会拉长持锁时间。
    // 查询失败按「可重投」处理：这不是配置问题，重投有意义。
    let expanded = expand_targets(db.as_ref(), msg, &message_msg.labels)
        .await
        .map_err(|e| NatsError::Handle(format!("展开投递目标失败: {e}")))?;
    info!(
        "投递目标展开完成: event_code={}, targets={}, skipped={}, 地址重复抑制={}",
        msg.event_code,
        expanded.targets.len(),
        expanded.skipped.len(),
        expanded.suppressed
    );

    // ── 5+6. 创建投递记录（唯一键即幂等判据） ──
    //
    // 这里**不做「先查一遍再插」**：幂等键就是 `msg_delivery` 上的唯一约束
    // `(event_code, business_id, business_trigger_ms)`，先 SELECT 只是把它抄
    // 了一遍，代价是多一次 RTT，还留着「查完到插入之间」的并发空窗——两个
    // 重复投递同时进来，两边都查不到、都去插，后到的撞键退化成 500 再重投。
    //
    // 直接让约束说话：插入成功就是第一次，撞唯一键就是已经投过。
    //
    // 投递记录、目标、渠道三张表必须同生共死：只落了投递记录就中断的话，重投
    // 会撞唯一键被当成「已投递」跳过，目标和渠道永远补不上，记录停在「投递中」。
    // 所以三张表放进同一个事务，失败整体回滚，提交之后才允许发布任务。

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
        // 与唯一键用的是同一个值，不能再各自兜底
        business_trigger_ms: Some(business_trigger_ms.into()),
        // 消息中心收到这条消息的时刻：渲染与写库都在这一瞬间之后，用它当接收
        // 时间戳，与业务触发时间之差就是发布方到消息中心的延迟
        receive_ms: Some(now_ms.into()),
        // 发布方给的链路第一跳接收时刻（服务端时钟），可能为空（没接入的发布方）
        first_receive_ms: Some(message_msg.first_receive_ms.map(Into::into)),
        // 发布方给的事件起点（业务时钟），可能为空（没接入的发布方、或本身没有事件可指）
        first_trigger_ms: Some(message_msg.first_trigger_ms.map(Into::into)),
        deliver_status: Some(DeliverStatus::Delivering),
        labels: Some(labels_json),
        annotations: Some(annotations_json),
        // 库列是 not null：走到这里标题内容已经校验过非空
        title: Some(title.clone()),
        content: Some(content.clone()),
        ext: Some(ext.clone()),
        remark: None,
        // 与 2.5 的模拟门控复用同一套标记识别，避免两处判定口径漂移
        simulated: Some(is_simulated(&message_msg)),
        // 审计列写真实落库时刻，不是 `now_ms`：那是「收到消息」的业务时刻，
        // 拿它回填会让 `create_ms` 与 `receive_ms` 恒等，渲染 + 写库的耗时
        // 就从库里消失了
        _current_ms: Some(U64(server_now_ms())),
        _current_user_id: U64(0),
    };

    let tx = db
        .begin()
        .await
        .map_err(|e| NatsError::Handle(format!("开启投递事务失败: {e}")))?;

    match MsgDeliverySvc::add(delivery_add, Some(&tx)).await {
        Ok(_) => {}
        // 撞幂等键：这条消息已经投过了（重投 / 两个消费者抢同一条）。
        // 正常路径，不是故障，日志降到 info 一行收掉。
        Err(SvcError::Dao(DaoError::DuplicateKey(key, _)))
            if key.table == DELIVERY_TABLE && key.key_name == DELIVERY_IDEM_KEY =>
        {
            // 失败的事务在 PG 里已经 aborted，先回滚再查。历史实现可能只落了
            // 投递记录、没来得及建目标/渠道，这里按当前配置补建并补发。
            let _ = tx.rollback().await;
            if backfill_delivery(&task_ctx, business_id, business_trigger_ms, &expanded).await? {
                info!(
                    "{trace} cost_ms={} result=idempotent_backfilled",
                    started.elapsed().as_millis()
                );
            } else {
                info!(
                    "消息已投递，幂等跳过: event_code={}, business_id={business_id}, business_trigger_ms={business_trigger_ms}",
                    message_msg.event_code
                );
                info!(
                    "{trace} cost_ms={} result=idempotent",
                    started.elapsed().as_millis()
                );
            }
            return Ok(());
        }
        Err(e) => {
            return Err(NatsError::Handle(format!(
                "创建投递记录失败 (business_id={business_id}): {e}"
            )));
        }
    }

    info!(
        "投递记录已创建: delivery_id={delivery_id}, message_id={}",
        msg.id
    );

    // ── 7+8. 创建投递目标与渠道记录（同一事务），并把投递任务收起来 ──

    let tasks = match create_targets_and_channels(&tx, delivery_id, &task_ctx, &expanded).await {
        Ok(tasks) => tasks,
        Err(e) => {
            // tx 在这里 drop，事务随之回滚：不会留下没有目标/渠道的投递记录
            return Err(NatsError::Handle(e));
        }
    };

    // 一个任务都没组装出来，说明这条消息谁都收不到：要么它没挂投递目标
    // （`msg_message_target` 缺行，比如发布方登记顺序不对），要么目标类别没绑
    // 渠道（`msg_target_category_channel` 缺行）。这两种情况下投递记录都不会
    // 再有人回写，会永远停在「投递中」——从后台看就是「已收下、没下文」，
    // 只能翻库才发现。趁事务还在就地收口成「全部失败」，避免提交后再回写失败
    // 又留下一条永远「投递中」的记录。
    //
    // 重投救不了：目标表没变，缓存重建后照样是零个任务。照常确认。
    if tasks.is_empty() {
        let reason = if msg.targets.is_empty() {
            "消息没有登记任何投递目标"
        } else {
            "投递目标下没有可用渠道（目标类别未绑定渠道）"
        };
        error!(
            "{reason}，本条消息判为投递失败: event_code={}, delivery_id={delivery_id}, business_id={business_id}",
            msg.event_code
        );

        MsgDeliverySvc::modify(
            MsgDeliveryModifyDto {
                id: Some(U64(delivery_id)),
                deliver_status: Some(DeliverStatus::AllFailed),
                _current_user_id: U64(0),
                ..Default::default()
            },
            Some(&tx),
        )
        .await
        .map_err(|e| NatsError::Handle(format!("回写投递失败状态失败: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| NatsError::Handle(format!("提交投递事务失败: {e}")))?;

        info!(
            "{trace} cost_ms={} result=no_task delivery_id={delivery_id}",
            started.elapsed().as_millis()
        );
        return Ok(());
    }

    // 记录先落定，提交后再发任务：消费者回写时要能查到渠道行
    tx.commit()
        .await
        .map_err(|e| NatsError::Handle(format!("提交投递事务失败: {e}")))?;

    // ── 9. 发布投递任务 ──

    let task_count = tasks.len();
    for task in tasks {
        if let Err(e) = wkr::publish_task(&task).await {
            error!(
                "发布投递任务失败: channel={}, target_id={}, error={e}",
                task.channel_code, task.target_id
            );
            // 发不出去就不会有发送结果回来，就地记失败，避免渠道一直挂在「投递中」。
            // 渠道日志的「接收时间」用这次发布尝试的时刻：任务根本没到消费端，
            // 这是最接近的事实。
            if let Err(write_err) = deliver_result::record(
                &task,
                &SendOutcome::Failed(format!("发布投递任务失败: {e}")),
                chrono::Utc::now().timestamp_millis() as u64,
            )
            .await
            {
                error!(
                    "发布投递任务失败后回写投递状态也失败: channel={}, target_id={}, error={write_err}",
                    task.channel_code, task.target_id
                );
            }
        }
    }

    info!(
        "消息处理完成: event_code={}, business_id={:?}",
        message_msg.event_code, message_msg.business_id
    );
    info!(
        "{trace} cost_ms={} result=ok delivery_id={delivery_id} tasks={}",
        started.elapsed().as_millis(),
        task_count
    );

    Ok(())
}

/// # 在事务里创建目标与渠道记录，返回待发布的任务
///
/// 必须由调用方提供事务连接：记录要么一起提交，要么一起回滚。渠道创建失败
/// 直接返回错误（回滚重投），不再像旧实现那样跳过——跳过会在目标下留下缺
/// 渠道的半截数据，那条目标永远等不到结果回写。
///
/// 入参是 [`ExpandOutcome`]：抽象目标（用户 / 角色 / 组织）已经展开成人 + 地址，
/// 这里只负责把结果落库。两类情况都落 `msg_delivery_target`：
///
/// - 能投的：状态 `Delivering`，渠道行带地址；
/// - 展不开的（0 人 / 全部无地址 / 用户已停用）：状态 `Skipped`，不建渠道行。
///   落这一行是为了让「这次投过这个类别、但没人可投」在库里可见，而不是静默消失。
///
/// 只跳过「渠道配置为空」的目标：没有渠道的目标不会有任务，也不会有结果回写，
/// 会永远停在「投递中」并拖住整条投递的收口。
async fn create_targets_and_channels<C>(
    db: &C,
    delivery_id: u64,
    task_ctx: &TaskContext<'_>,
    expanded: &ExpandOutcome,
) -> Result<Vec<DeliveryMsg>, String>
where
    C: ConnectionTrait,
{
    let mut tasks = Vec::new();

    // ── 1. 展不开的目标：落 Skipped，原因进日志 ──
    for skip in &expanded.skipped {
        let target_id = next_id().map_err(|e| format!("生成投递目标ID失败: {e}"))?;

        let target_add = MsgDeliveryTargetAddDto {
            id: Some(U64(target_id)),
            delivery_id: Some(delivery_id.into()),
            target_category_id: Some(skip.category_id.into()),
            target_id: Some(skip.target_id.clone()),
            origin: Some(skip.origin.clone()),
            deliver_target_status: Some(DeliverTargetStatus::Skipped),
            _current_ms: Some(U64(server_now_ms())),
            _current_user_id: U64(0),
        };

        MsgDeliveryTargetSvc::add(target_add, Some(db))
            .await
            .map_err(|e| {
                format!(
                    "创建 Skipped 投递目标失败 (category_code={}, target_id={}): {e}",
                    skip.category_code, skip.target_id
                )
            })?;

        warn!(
            "投递目标未展开出可投目标，已记 Skipped: delivery_id={delivery_id}, target_id={target_id}, category_code={}, target_id_raw={}, 原因={}",
            skip.category_code, skip.target_id, skip.reason
        );
    }

    // ── 2. 可投目标：落目标行 + 渠道行，组装任务 ──
    for target in &expanded.targets {
        if target.channels.is_empty() {
            error!(
                "目标未配渠道，跳过该目标（请在消息目标上补渠道）: message={}, category_code={}, target_id={}",
                task_ctx.msg.event_code, target.category_code, target.target_id
            );
            continue;
        }

        let target_id = next_id().map_err(|e| format!("生成投递目标ID失败: {e}"))?;

        let target_add = MsgDeliveryTargetAddDto {
            id: Some(U64(target_id)),
            delivery_id: Some(delivery_id.into()),
            target_category_id: Some(target.category_id.into()),
            // 展开后 target_id 就是最终目标：人=用户ID，大屏=大屏标识，定位卡=卡号
            target_id: Some(target.target_id.clone()),
            origin: Some(target.origin.clone()),
            deliver_target_status: Some(DeliverTargetStatus::Delivering),
            _current_ms: Some(U64(server_now_ms())),
            _current_user_id: U64(0),
        };

        MsgDeliveryTargetSvc::add(target_add, Some(db))
            .await
            .map_err(|e| {
                format!(
                    "创建投递目标失败 (category_code={}, target_id={}): {e}",
                    target.category_code, target.target_id
                )
            })?;

        info!(
            "投递目标已创建: target_id={target_id}, category_code={}, target_key={}, origin={:?}, delivery_id={delivery_id}",
            target.category_code, target.target_id, target.origin
        );

        for route in &target.channels {
            let channel_id = next_id().map_err(|e| format!("生成投递渠道ID失败: {e}"))?;

            let channel_add = MsgDeliveryChannelAddDto {
                id: Some(U64(channel_id)),
                deliver_target_id: Some(U64(target_id)),
                channel_id: Some(route.channel.id.into()),
                deliver_channel_status: Some(DeliverChannelStatus::Delivering),
                // 展开时已取好地址（用户手机号优先、人员兜底），发送器直接用
                address: Some(route.address.clone()),
                _current_ms: Some(U64(server_now_ms())),
                _current_user_id: U64(0),
            };

            MsgDeliveryChannelSvc::add(channel_add, Some(db))
                .await
                .map_err(|e| {
                    format!(
                        "创建投递渠道失败 (target_id={target_id}, channel_id={}): {e}",
                        route.channel.id
                    )
                })?;

            info!(
                "投递渠道已创建: target_id={target_id}, channel_id={}, channel_name={}, address={:?}",
                route.channel.id, route.channel.name, route.address
            );

            tasks.push(build_task(
                task_ctx,
                (Some(delivery_id), Some(target_id), Some(channel_id)),
                target,
                route,
            ));
        }
    }

    Ok(tasks)
}

/// # 幂等键冲突时补齐既有投递的目标/渠道
///
/// 历史实现先落投递记录、再建目标/渠道，中途失败会留下「有投递、没目标」的
/// 半截数据；旧逻辑重投时直接按幂等跳过，消息就永远停在「投递中」。这里按
/// 当前缓存的目标配置检查既有投递：缺目标补目标、缺渠道补渠道，只把补出来的
/// 那几路发出去。已有渠道不重发——那属于「再次投递」，不由幂等冲突触发。
///
/// 返回 `Ok(true)` 表示补建过并已发任务，`Ok(false)` 表示既有记录本来就完整。
async fn backfill_delivery(
    task_ctx: &TaskContext<'_>,
    business_id: u64,
    business_trigger_ms: u64,
    expanded: &ExpandOutcome,
) -> Result<bool, NatsError> {
    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;
    let tx = db
        .begin()
        .await
        .map_err(|e| NatsError::Handle(format!("开启补建事务失败: {e}")))?;

    let delivery = MsgDeliverySvc::list_by_query_dto(
        MsgDeliveryQueryDto {
            event_code: Some(task_ctx.msg.event_code.clone()),
            business_id: Some(business_id.into()),
            business_trigger_ms: Some(business_trigger_ms.into()),
            ..Default::default()
        },
        Some(&tx),
    )
    .await
    .map_err(|e| NatsError::Handle(format!("查询既有投递记录失败: {e}")))?
    .extra
    .unwrap_or_default()
    .into_iter()
    .next()
    .ok_or_else(|| NatsError::Handle("幂等键冲突但查不到既有投递记录，等待重投".to_string()))?;

    let targets = MsgDeliveryTargetSvc::list_by_query_dto(
        MsgDeliveryTargetQueryDto {
            delivery_id: Some(delivery.id),
            ..Default::default()
        },
        Some(&tx),
    )
    .await
    .map_err(|e| NatsError::Handle(format!("查询既有投递目标失败: {e}")))?
    .extra
    .unwrap_or_default();

    // 补建出来的任务沿用原投递的创建时刻：历史事件不该在大屏上显示成「刚刚」
    let backfill_ctx = TaskContext {
        msg: task_ctx.msg,
        message_msg: task_ctx.message_msg,
        title: task_ctx.title,
        content: task_ctx.content,
        ext: task_ctx.ext,
        create_ms: delivery.create_ms.0,
    };

    let mut tasks = Vec::new();
    let mut repaired = false;

    for target in &expanded.targets {
        if target.channels.is_empty() {
            continue;
        }

        // 目标标识已经是展开后的最终目标（人=用户ID），按它比对既有行
        let existing_target = targets.iter().find(|row| {
            row.target_category_id == U64(target.category_id) && row.target_id == target.target_id
        });

        let target_id = match existing_target {
            Some(row) => row.id.0,
            None => {
                let target_id =
                    next_id().map_err(|e| NatsError::Handle(format!("生成投递目标ID失败: {e}")))?;
                let target_add = MsgDeliveryTargetAddDto {
                    id: Some(U64(target_id)),
                    delivery_id: Some(delivery.id),
                    target_category_id: Some(target.category_id.into()),
                    target_id: Some(target.target_id.clone()),
                    origin: Some(target.origin.clone()),
                    deliver_target_status: Some(DeliverTargetStatus::Delivering),
                    _current_ms: Some(U64(server_now_ms())),
                    _current_user_id: U64(0),
                };
                MsgDeliveryTargetSvc::add(target_add, Some(&tx))
                    .await
                    .map_err(|e| {
                        NatsError::Handle(format!(
                            "补建投递目标失败 (category_code={}): {e}",
                            target.category_code
                        ))
                    })?;
                repaired = true;
                target_id
            }
        };

        // 目标已存在时先看它已经挂了哪些渠道，避免重复补建同一路
        let existing_channels = if existing_target.is_some() {
            MsgDeliveryChannelSvc::list_by_query_dto(
                MsgDeliveryChannelQueryDto {
                    deliver_target_id: Some(U64(target_id)),
                    ..Default::default()
                },
                Some(&tx),
            )
            .await
            .map_err(|e| NatsError::Handle(format!("查询既有投递渠道失败: {e}")))?
            .extra
            .unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut added_channel = false;
        for route in &target.channels {
            if existing_channels
                .iter()
                .any(|row| row.channel_id == U64(route.channel.id))
            {
                continue;
            }

            let channel_id =
                next_id().map_err(|e| NatsError::Handle(format!("生成投递渠道ID失败: {e}")))?;
            let channel_add = MsgDeliveryChannelAddDto {
                id: Some(U64(channel_id)),
                deliver_target_id: Some(U64(target_id)),
                channel_id: Some(route.channel.id.into()),
                deliver_channel_status: Some(DeliverChannelStatus::Delivering),
                address: Some(route.address.clone()),
                _current_ms: Some(U64(server_now_ms())),
                _current_user_id: U64(0),
            };
            MsgDeliveryChannelSvc::add(channel_add, Some(&tx))
                .await
                .map_err(|e| {
                    NatsError::Handle(format!(
                        "补建投递渠道失败 (target_id={target_id}, channel_id={}): {e}",
                        route.channel.id
                    ))
                })?;

            tasks.push(build_task(
                &backfill_ctx,
                (Some(delivery.id.0), Some(target_id), Some(channel_id)),
                target,
                route,
            ));
            added_channel = true;
            repaired = true;
        }

        // 给已有目标补了渠道：目标可能停在「失败」，拉回「投递中」等新任务回写
        if added_channel && existing_target.is_some() {
            MsgDeliveryTargetSvc::modify(
                MsgDeliveryTargetModifyDto {
                    id: Some(U64(target_id)),
                    deliver_target_status: Some(DeliverTargetStatus::Delivering),
                    _current_user_id: U64(0),
                    ..Default::default()
                },
                Some(&tx),
            )
            .await
            .map_err(|e| NatsError::Handle(format!("重置投递目标状态失败: {e}")))?;
        }
    }

    if !repaired {
        // 既有投递本来就是完整的，真正的重复消息：事务里什么都没写，直接回滚
        let _ = tx.rollback().await;
        return Ok(false);
    }

    // 半截数据已补齐，把整条投递拉回「投递中」，等新旧任务一起收口
    MsgDeliverySvc::modify(
        MsgDeliveryModifyDto {
            id: Some(delivery.id),
            deliver_status: Some(DeliverStatus::Delivering),
            _current_user_id: U64(0),
            ..Default::default()
        },
        Some(&tx),
    )
    .await
    .map_err(|e| NatsError::Handle(format!("重置投递状态失败: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| NatsError::Handle(format!("提交补建事务失败: {e}")))?;

    let task_count = tasks.len();
    for task in &tasks {
        if let Err(e) = wkr::publish_task(task).await {
            error!(
                "补建后发布投递任务失败: channel={}, target_id={}, error={e}",
                task.channel_code, task.target_id
            );
            if let Err(write_err) = deliver_result::record(
                task,
                &SendOutcome::Failed(format!("发布投递任务失败: {e}")),
                chrono::Utc::now().timestamp_millis() as u64,
            )
            .await
            {
                error!(
                    "补建后发布失败且回写状态也失败: channel={}, target_id={}, error={write_err}",
                    task.channel_code, task.target_id
                );
            }
        }
    }

    info!(
        "幂等冲突的既有投递已补建: delivery_id={}, tasks={task_count}",
        delivery.id.0
    );
    Ok(true)
}

/// `msg_delivery.title` 列宽，也是 `MsgDeliveryDto.title` 的校验上限
const TITLE_MAX_CHARS: usize = 150;

/// # 按字符截断标题
///
/// 用 `char_indices` 而不是 `String::truncate(len)`：后者按字节切，中文会被
/// 切成非法 UTF-8 片段直接 panic。截断只影响展示，完整内容仍在 `content`。
fn truncate_title(mut title: String) -> String {
    if let Some((cut, _)) = title.char_indices().nth(TITLE_MAX_CHARS) {
        let preview: String = title.chars().take(50).collect();
        warn!(
            "标题超过 {TITLE_MAX_CHARS} 字符，已截断: 原长度={}, preview={preview:?}",
            title.chars().count()
        );
        title.truncate(cut);
    }
    title
}

/// # 消息是否带模拟标记
///
/// 发布方把模拟标记写在 `labels` / `annotations` 里，只认 `simulated=true`：
/// 值做去空白、忽略大小写比较，两份里任意一份命中即算模拟。
///
/// 早期版本还有一个等价别名，写入侧与读取侧都已移除，**不要再加回兼容分支**：
/// 存量报警快照由迁移统一补齐 `simulated=true`，旧版本发布方不再被识别。
pub(crate) fn is_simulated(message_msg: &MessageMsg) -> bool {
    let matches = |map: &HashMap<String, String>| {
        map.get("simulated")
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
    };
    matches(&message_msg.labels) || matches(&message_msg.annotations)
}

/// # 把 `simulated=true` 写进大屏读的那份 `ext`
///
/// `ext` 是「格式不约定、接收方自己解」的载荷槽，大屏解析的是这段 JSON。
/// 模拟标记原本只在 `labels` / `annotations` 里，而这两份只是模板输入，不随
/// 投递下发；在渲染收口之后补一次，大屏就能按同一个键判断「这是演示数据」，
/// 不必按事件类型分叉，也不依赖任何策略模板是否升级过。
///
/// 值保持字符串 `"true"`，与发布方 labels/annotations 里的写法一致，避免大屏
/// 再处理布尔与字符串两种形态。原本就是 JSON 对象时并进去（键已存在则覆盖成
/// `"true"`，防止模板自己写了 `null`）；原本没有 `ext` 时补一份最小对象。
///
/// 渲染结果不是 JSON 对象（数组、标量或压根不是 JSON）时不硬塞：强行包一层会
/// 改掉接收方认识的形状，只告警并原样发。这种形状本来就少见，且不该为了一个
/// 标记把整帧变成大屏解析不了的东西。
fn inject_simulated_ext(ext: Option<&str>) -> Option<String> {
    let marker = serde_json::Value::String("true".to_string());
    let Some(text) = ext else {
        return Some(serde_json::json!({ "simulated": "true" }).to_string());
    };

    match serde_json::from_str::<serde_json::Value>(text) {
        Ok(serde_json::Value::Object(mut map)) => {
            map.insert("simulated".to_string(), marker);
            Some(serde_json::Value::Object(map).to_string())
        }
        Ok(_) => {
            warn!("模拟消息的 ext 不是 JSON 对象，simulated 标记未并入: {text}");
            Some(text.to_string())
        }
        Err(e) => {
            warn!("模拟消息的 ext 不是合法 JSON，simulated 标记未并入: {e}, ext={text}");
            Some(text.to_string())
        }
    }
}

/// # 组装投递任务时共用的上下文
///
/// 持久化与非持久化两条路径只有「三张表的行 ID 有没有」不同，其余都取自这里。
struct TaskContext<'a> {
    msg: &'a MessageCache,
    message_msg: &'a MessageMsg,
    /// 渲染结果，已校验非空
    title: &'a str,
    /// 渲染结果，已校验非空
    content: &'a str,
    /// 渲染结果，可为空
    ext: Option<&'a str>,
    create_ms: u64,
}

/// # 组装模板的内置变量
///
/// `_` 前缀是保留命名空间：这些值是服务端知道的事实（处理时间、业务触发
/// 时间、消息自身标识），发布方在 `labels` 里写同名键会被忽略。模板里
/// `{{_create_ms:time}}` 直接可用，不必为了打时间戳再塞一份进 `labels`。
///
/// 可选值（发布方没给）就不注入，模板里缺变量时按缺失处理——`default`
/// 兜底、`json` 输出 `null`，与其他缺失变量一致。
fn template_builtins(
    msg: &MessageCache,
    message_msg: &MessageMsg,
    create_ms: u64,
) -> HashMap<&'static str, String> {
    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("_create_ms", create_ms.to_string()),
        ("_message_id", msg.id.to_string()),
        ("_event_code", msg.event_code.clone()),
    ]);

    if let Some(business_id) = message_msg.business_id {
        builtins.insert("_business_id", business_id.to_string());
    }
    if let Some(trigger_ms) = message_msg.business_trigger_ms {
        builtins.insert("_business_trigger_ms", trigger_ms.to_string());
    }

    builtins
}

/// # 按优先级取一个已渲染的字段
///
/// 模板有值就用模板（运营配置的优先级高于发布方），模板为 `null` 才用发布方
/// 给的值。渲染结果全是空白等同于没有——空串会让消费方的 `is_some()` 判断
/// 失效，也过不了标题内容的非空校验。
fn pick(
    template: Option<&str>,
    from_publisher: Option<&str>,
    builtins: &HashMap<&'static str, String>,
    labels: &HashMap<String, String>,
    annotations: &HashMap<String, String>,
) -> Option<String> {
    let rendered = match template {
        Some(template) => render_template(template, builtins, labels, annotations),
        None => from_publisher?.to_string(),
    };

    if rendered.trim().is_empty() {
        None
    } else {
        Some(rendered)
    }
}

/// # 组装一条投递任务
///
/// `ids` 是 `(msg_delivery.id, msg_delivery_target.id, msg_delivery_channel.id)`：
/// 持久化消息带真实行 ID，非持久化消息全为 `None`。其余字段直接对应投递三张表
/// 的列，不做额外加工。
fn build_task(
    ctx: &TaskContext<'_>,
    ids: (Option<u64>, Option<u64>, Option<u64>),
    target: &ExpandedTarget,
    route: &ExpandedChannel,
) -> DeliveryMsg {
    DeliveryMsg {
        persisted: ctx.msg.persisted,
        delivery_id: ids.0,
        delivery_target_id: ids.1,
        delivery_channel_id: ids.2,
        // 下发的是发布方的事件编码，不是模板编码：接收方按事件编码渲染/路由，
        // 「模板编码 ≠ 事件编码」时拿到的仍是发布方的真实事件。
        event_code: ctx.message_msg.event_code.clone(),
        message_id: ctx.msg.id,
        event_source_id: ctx.msg.source.id,
        message_category_id: ctx.msg.category.id,
        business_id: ctx.message_msg.business_id,
        business_trigger_ms: ctx.message_msg.business_trigger_ms,
        // 事件起点原样带下去：渠道日志用它算「事件发生 → 送达」，发布方没给就是
        // None。它来自业务/发布方时钟，可能不准。
        first_trigger_ms: ctx.message_msg.first_trigger_ms,
        // 链路第一跳收到事件的服务端时刻，也原样带下去：端到端延迟只信它
        first_receive_ms: ctx.message_msg.first_receive_ms,
        title: ctx.title.to_string(),
        content: ctx.content.to_string(),
        ext: ctx.ext.map(str::to_string),
        create_ms: ctx.create_ms,
        // 模拟标记随任务一路带下去：投递任务失败进死信时，死信表直接取这个值
        simulated: is_simulated(ctx.message_msg),
        // 首次投递，不是手工重投
        redeliver: false,
        target_category_code: target.category_code.clone(),
        target_category_id: target.category_id,
        target_id: target.target_id.clone(),
        channel_id: route.channel.id,
        channel_code: route.channel.code.clone(),
        channel_type: route.channel.channel_type.clone(),
        // 短信 / 邮件要的号码或邮箱，展开时已解析好；大屏、定位卡为 None
        address: route.address.clone(),
    }
}

/// # 非持久化消息的投递
///
/// 不写任何表、不做幂等、不回写状态：按展开后的目标 × 渠道直接发，
/// 发不出去只打日志。这类消息语义就是 at-most-once。
///
/// 展不开的目标（0 人 / 全部无地址）在这里没有表可落，只打一条 warn：
/// 非持久化消息本来就「发不出去就是丢了」，但要让人知道丢在哪一步。
async fn publish_best_effort(
    msg: &MessageCache,
    message_msg: &MessageMsg,
    title: &str,
    content: &str,
    ext: Option<&str>,
    create_ms: u64,
    expanded: &ExpandOutcome,
) {
    if msg.targets.is_empty() {
        error!(
            "消息 {} 没有登记任何投递目标，本条消息丢弃: business_id={:?}",
            msg.event_code, message_msg.business_id
        );
        return;
    }

    for skip in &expanded.skipped {
        warn!(
            "非持久化消息有目标展不开，已丢弃: event_code={}, category_code={}, target_raw={}, 原因={}",
            msg.event_code, skip.category_code, skip.target_id, skip.reason
        );
    }

    if expanded.targets.is_empty() {
        error!(
            "消息 {} 没有可投目标，本条消息丢弃: business_id={:?}",
            msg.event_code, message_msg.business_id
        );
        return;
    }

    let ctx = TaskContext {
        msg,
        message_msg,
        title,
        content,
        ext,
        create_ms,
    };

    for target in &expanded.targets {
        for route in &target.channels {
            let task = build_task(&ctx, (None, None, None), target, route);
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

    /// 模板优先于发布方：模板是运营配的，发布方给的值只在模板缺席时兜底
    #[test]
    fn pick_prefers_template_over_publisher() {
        let labels = HashMap::from([("name".to_string(), "张三".to_string())]);
        let annotations = HashMap::new();
        let builtins = HashMap::new();

        assert_eq!(
            pick(
                Some("你好 {{name}}"),
                Some("发布方的标题"),
                &builtins,
                &labels,
                &annotations
            ),
            Some("你好 张三".to_string())
        );
        assert_eq!(
            pick(None, Some("发布方的标题"), &builtins, &labels, &annotations),
            Some("发布方的标题".to_string())
        );
        assert_eq!(pick(None, None, &builtins, &labels, &annotations), None);
    }

    /// 渲染出空白等同于没有：空串会骗过 is_some()，也过不了非空校验
    #[test]
    fn pick_treats_blank_as_absent() {
        let builtins: HashMap<&'static str, String> = HashMap::new();
        let labels: HashMap<String, String> = HashMap::new();
        let annotations: HashMap<String, String> = HashMap::new();

        // 模板配了但渲染出空白：算没有，不回落到发布方（模板已接管这个字段）
        assert_eq!(
            pick(Some("   "), Some("有值"), &builtins, &labels, &annotations),
            None
        );
        assert_eq!(pick(None, Some(""), &builtins, &labels, &annotations), None);
    }

    /// 标题按字符截断：150 字符是列宽，恰好 150 不动，超一个字符才截。
    ///
    /// 用中文验证「按字符不是按字节」：按字节切会落在 UTF-8 中间而 panic，
    /// 或者留下坏字符。
    #[test]
    fn truncate_title_keeps_limit_and_counts_chars() {
        let exact = "中".repeat(TITLE_MAX_CHARS);
        assert_eq!(truncate_title(exact.clone()), exact);

        let long = "警".repeat(TITLE_MAX_CHARS + 10);
        let truncated = truncate_title(long);
        assert_eq!(truncated.chars().count(), TITLE_MAX_CHARS);
        assert!(truncated.chars().all(|c| c == '警'));
    }

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
        let token = nats_name_token("msg.message.normal");
        assert!(is_valid_nats_name(&token), "仍含非法字符: {token}");
        assert!(token.starts_with("msg-message-normal-"), "token={token}");
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

    /// 只认 simulated=true：真实消息与只带旧别名的消息都不命中
    #[test]
    fn simulated_marker_is_recognized() {
        let message = |labels: &[(&str, &str)], annotations: &[(&str, &str)]| {
            MessageMsg::builder()
                .event_code("alarm.region".to_string())
                .business_id(None)
                .business_trigger_ms(None)
                .labels(
                    labels
                        .iter()
                        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                        .collect(),
                )
                .annotations(
                    annotations
                        .iter()
                        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                        .collect(),
                )
                .build()
        };

        assert!(is_simulated(&message(&[("simulated", "true")], &[])));
        assert!(is_simulated(&message(&[], &[("simulated", "true")])));
        // 值做去空白、忽略大小写比较
        assert!(is_simulated(&message(&[("simulated", " TRUE ")], &[])));
        // 真实通知：没有标记，或者标记不是模拟
        assert!(!is_simulated(&message(&[], &[])));
        assert!(!is_simulated(&message(&[("simulated", "false")], &[])));
        // 早期别名已彻底废弃：只带它的消息一律按真实通知处理
        assert!(!is_simulated(&message(&[("source", "sim")], &[])));
        assert!(!is_simulated(&message(&[], &[("source", " SIM ")])));
    }

    /// 模拟标记随 ext 下发：大屏解析的那段 JSON 里能读到 simulated=true，
    /// 其余键一位不改；模板自己渲染出的 `null` 会被覆盖成 `"true"`。
    #[test]
    fn simulated_marker_is_injected_into_ext() {
        let ext = r#"{"alertId":"7","simulated":null,"unityX":"1.5"}"#;
        let marked = inject_simulated_ext(Some(ext)).expect("模拟消息必须有 ext");
        let value: serde_json::Value =
            serde_json::from_str(&marked).expect("ext 必须仍是合法 JSON");
        assert_eq!(value["simulated"], "true");
        assert_eq!(value["alertId"], "7");
        assert_eq!(value["unityX"], "1.5");

        // 原本没有 ext 的模拟消息也补一份最小对象，不能让大屏收不到标记
        let created = inject_simulated_ext(None).expect("模拟消息必须有 ext");
        let value: serde_json::Value =
            serde_json::from_str(&created).expect("ext 必须仍是合法 JSON");
        assert_eq!(value, serde_json::json!({ "simulated": "true" }));
    }

    /// ext 形状认不出来时原样发，不强行包一层改掉接收方认识的形状
    #[test]
    fn unparsable_ext_is_left_alone() {
        assert_eq!(
            inject_simulated_ext(Some("不是 JSON")).as_deref(),
            Some("不是 JSON")
        );
        assert_eq!(
            inject_simulated_ext(Some("[1,2]")).as_deref(),
            Some("[1,2]")
        );
    }

    /// 真实消息不走注入分支：调用方只在 is_simulated 命中时才调，
    /// 这里钉住「无标记 = 不命中」这一侧，保证真实报警/位置的 ext 不变
    #[test]
    fn real_message_ext_is_untouched() {
        let message = MessageMsg::builder()
            .event_code("alarm.region".to_string())
            .business_id(None)
            .business_trigger_ms(None)
            .build();
        assert!(!is_simulated(&message));
    }
}
