//! # 消息发布核心逻辑
//!
//! 通过 msg-api-client 的 Feign 客户端调用 msg-svr 完成模板查询、渲染和投递创建，
//! 以及消息队列、事件来源、消息类别的注册。

use crate::config::get_api_config;
use crate::utils::msg_pub_error::MsgPubError;
use msg_api::cst::MSG_MESSAGE_TOPIC;
use msg_api::mqo::message_mqo::MessageMqo;
use robotech::api::U64;
use robotech::micro_svc::FeignApiClient;
use robotech::mq::nats::publish;

pub async fn publish_message(message_mqo: MessageMqo) -> Result<(), MsgPubError> {
    publish(MSG_MESSAGE_TOPIC, &message_mqo, false).await?;
    Ok(())
}

/// # 注册消息队列
///
/// 通过 feign 客户端调用 msg-svr 的 `MsgMessageQueueSvc::add`，
/// 写入成功后 msg-svr 会通过 `after_write` 回调自动刷新队列缓存。
///
/// ## 参数
///
/// - `code`: 队列编码，用于 NATS subject 绑定
/// - `name`: 队列名称
/// - `persisted`: 是否持久化（`true` → JetStream，`false` → Core NATS）
/// - `remark`: 备注（可选）
///
/// ## 返回值
///
/// 成功返回 `Ok(())`，失败返回 `MsgPubError::Api`。
pub async fn register_message_queue(
    code: String,
    name: String,
    persisted: bool,
    remark: Option<String>,
) -> Result<(), MsgPubError> {
    let cfg = get_api_config().map_err(|e| {
        MsgPubError::Api(robotech::api_client::ApiClientError::GetApiClient(
            e.to_string(),
        ))
    })?;
    let client = FeignApiClient::new((*cfg).clone()).await;
    let queue_client = msg_api_client::MsgMessageQueueApiClient::new(client);
    let dto = msg_api_client::dto::msg_message_queue_dto::MsgMessageQueueAddDto::builder()
        .code(code)
        .name(name)
        .persisted(persisted)
        .remark(remark)
        ._current_user_id(U64(0))
        .build();
    queue_client.add(&dto).await?;
    Ok(())
}

/// # 注册消息事件来源
///
/// 通过 feign 客户端调用 msg-svr 的 `MsgEventSourceSvc::add`，
/// 写入成功后 msg-svr 会通过 `after_write` 回调自动刷新消息缓存。
///
/// ## 参数
///
/// - `code`: 来源编码
/// - `name`: 来源名称
/// - `remark`: 备注（可选）
///
/// ## 返回值
///
/// 成功返回 `Ok(())`，失败返回 `MsgPubError::Api`。
pub async fn register_event_source(
    code: String,
    name: String,
    remark: Option<String>,
) -> Result<(), MsgPubError> {
    let cfg = get_api_config().map_err(|e| {
        MsgPubError::Api(robotech::api_client::ApiClientError::GetApiClient(
            e.to_string(),
        ))
    })?;
    let client = FeignApiClient::new((*cfg).clone()).await;
    let source_client = msg_api_client::MsgEventSourceApiClient::new(client);
    let dto = msg_api_client::dto::msg_event_source_dto::MsgEventSourceAddDto::builder()
        .code(code)
        .name(name)
        .remark(remark)
        ._current_user_id(U64(0))
        .build();
    source_client.add(&dto).await?;
    Ok(())
}

/// # 注册消息类别
///
/// 通过 feign 客户端调用 msg-svr 的 `MsgMessageCategorySvc::add`，
/// 写入成功后 msg-svr 会通过 `after_write` 回调自动刷新消息缓存。
///
/// ## 参数
///
/// - `code`: 类别编码
/// - `name`: 类别名称
/// - `remark`: 备注（可选）
///
/// ## 返回值
///
/// 成功返回 `Ok(())`，失败返回 `MsgPubError::Api`。
pub async fn register_message_category(
    code: String,
    name: String,
    remark: Option<String>,
) -> Result<(), MsgPubError> {
    let cfg = get_api_config().map_err(|e| {
        MsgPubError::Api(robotech::api_client::ApiClientError::GetApiClient(
            e.to_string(),
        ))
    })?;
    let client = FeignApiClient::new((*cfg).clone()).await;
    let category_client = msg_api_client::MsgMessageCategoryApiClient::new(client);
    let dto =
        msg_api_client::dto::msg_message_category_dto::MsgMessageCategoryAddDto::builder()
            .code(code)
            .name(name)
            .remark(remark)
            ._current_user_id(U64(0))
            .build();
    category_client.add(&dto).await?;
    Ok(())
}

/// # 注册消息
///
/// 通过 feign 客户端调用 msg-svr 的 `MsgMessageSvc::add`，
/// 写入成功后 msg-svr 会通过 `after_write` 回调自动刷新消息缓存。
///
/// ## 参数
///
/// - `category_id`: 消息类别ID
/// - `message_queue_id`: 消息队列ID
/// - `event_source_id`: 事件来源ID
/// - `name`: 消息名称
/// - `event_code`: 事件编码（在应用中定义，事件触发时传递出来）
/// - `persisted`: 是否持久化（可选，默认 `true`）
/// - `title_template`: 标题模板（可选）
/// - `content_template`: 内容模板（可选）
/// - `remark`: 备注（可选）
/// - `enabled`: 是否启用（可选，默认 `true`）
///
/// ## 返回值
///
/// 成功返回 `Ok(())`，失败返回 `MsgPubError::Api`。
pub async fn register_message(
    category_id: u64,
    message_queue_id: u64,
    event_source_id: u64,
    name: String,
    event_code: String,
    persisted: Option<bool>,
    title_template: Option<String>,
    content_template: Option<String>,
    remark: Option<String>,
    enabled: Option<bool>,
) -> Result<(), MsgPubError> {
    let cfg = get_api_config().map_err(|e| {
        MsgPubError::Api(robotech::api_client::ApiClientError::GetApiClient(
            e.to_string(),
        ))
    })?;
    let client = FeignApiClient::new((*cfg).clone()).await;
    let message_client = msg_api_client::MsgMessageApiClient::new(client);
    let dto = msg_api_client::dto::msg_message_dto::MsgMessageAddDto::builder()
        .category_id(U64(category_id))
        .message_queue_id(U64(message_queue_id))
        .event_source_id(U64(event_source_id))
        .name(name)
        .event_code(event_code)
        .persisted(persisted.unwrap_or(true))
        .title_template(title_template)
        .content_template(content_template)
        .remark(remark)
        .enabled(enabled.unwrap_or(true))
        ._current_user_id(U64(0))
        .build();
    message_client.add(&dto).await?;
    Ok(())
}