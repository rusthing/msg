//! # 死信订阅
//!
//! 消费 `dlq.>` 上的消息并落库到 `msg_dead_letter`，供人工排查与重放。
//!
//! ## 数据流
//!
//! ```text
//! 业务消息重投达到 max_deliver
//!   → robotech 把原始负载 + 元信息发到 dlq.{原subject}
//!   → 本订阅消费 → 写入 msg_dead_letter
//! ```
//!
//! 用 JetStream 而不是 core NATS：死信必须持久化，否则没有订阅者时会被直接丢弃。

use crate::config::get_msg_config;
use crate::dto::MsgDeadLetterAddDto;
use crate::svc::MsgDeadLetterSvc;
use arc_swap::ArcSwapOption;
use async_nats::jetstream::{consumer, stream};
use idworker::next_id;
use robotech::api::U64;
use robotech::db::get_db_conn;
use robotech::mq::nats;
use robotech::mq::nats::NatsError;
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{info, warn};

/// 死信主题前缀，与 `robotech::mq::nats::DLQ_SUBJECT_PREFIX` 保持一致
const DLQ_PREFIX: &str = "dlq.";

/// 死信 Stream 名称
const DLQ_STREAM: &str = "MSG-DLQ";

/// 死信 Consumer 名称
const DLQ_CONSUMER: &str = "msg-svr-dlq";

/// 死信队列组，多副本之间负载均衡
const DLQ_QUEUE_GROUP: &str = "msg-svr-dlq";

/// 死信保留时长
const DLQ_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 3600);

/// 当前死信订阅句柄
static DLQ_SUBSCRIBER: ArcSwapOption<JoinHandle<()>> = ArcSwapOption::const_empty();

/// # 启动死信订阅
///
/// 幂等：重复调用会先停掉旧订阅再重建。配置热更新会换掉 NATS Client，
/// 旧订阅随之失效，所以每次 `setup` 都要重新建立。
pub async fn setup_dead_letter_subscription() -> anyhow::Result<()> {
    if let Some(old) = DLQ_SUBSCRIBER.swap(None) {
        old.abort();
    }

    let stream_config = stream::Config {
        name: DLQ_STREAM.to_string(),
        subjects: vec![format!("{DLQ_PREFIX}>")],
        max_age: DLQ_MAX_AGE,
        ..Default::default()
    };

    let consumer_config = consumer::push::Config {
        durable_name: Some(DLQ_CONSUMER.to_string()),
        deliver_policy: consumer::DeliverPolicy::All,
        ack_policy: consumer::AckPolicy::Explicit,
        // 落库失败重试次数有限，再失败说明表本身也有问题——次数由配置
        // `msg.dead-letter-max-deliver` 控制
        max_deliver: get_msg_config().effective_dead_letter_max_deliver(),
        ..Default::default()
    };

    let handle = nats::subscribe(
        &format!("{DLQ_PREFIX}>"),
        Some(DLQ_QUEUE_GROUP),
        Some(stream_config),
        Some(consumer_config),
        handle_dead_letter,
    )
    .await?;

    DLQ_SUBSCRIBER.store(Some(handle));
    info!("死信订阅已启动 (stream={DLQ_STREAM})");
    Ok(())
}

/// # 处理一条死信
///
/// 原主题、原因、投递次数从 header 取，负载原样落库。
async fn handle_dead_letter(msg: async_nats::Message) -> Result<(), NatsError> {
    let subject = header_value(&msg, "Nats-Dlq-Subject").unwrap_or_else(|| {
        msg.subject
            .as_str()
            .strip_prefix(DLQ_PREFIX)
            .unwrap_or(msg.subject.as_str())
            .to_string()
    });
    let reason = header_value(&msg, "Nats-Dlq-Reason").unwrap_or_else(|| "未知原因".to_string());
    let delivered = header_value(&msg, "Nats-Dlq-Delivered")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);

    warn!("收到死信: subject={subject}, delivered={delivered}, reason={reason}");

    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let id = next_id().map_err(|e| NatsError::Handle(format!("生成死信ID失败: {e}")))?;

    let add = MsgDeadLetterAddDto {
        id: Some(U64(id)),
        subject: Some(subject),
        reason: Some(reason),
        payload: Some(String::from_utf8_lossy(&msg.payload).to_string()),
        delivered: Some(U64(delivered)),
        handled: Some(false),
        remark: None,
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
    };

    MsgDeadLetterSvc::add(add, Some(db.as_ref()))
        .await
        .map_err(|e| NatsError::Handle(format!("写入死信记录失败: {e}")))?;

    info!("死信已落库: id={id}");
    Ok(())
}

/// 取 header 的最后一个值（单值时就是唯一值）
fn header_value(msg: &async_nats::Message, key: &str) -> Option<String> {
    msg.headers
        .as_ref()?
        .get_last(key)
        .map(|value| value.as_str().to_string())
}
