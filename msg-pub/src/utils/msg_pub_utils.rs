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