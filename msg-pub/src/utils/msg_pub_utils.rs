//! # 消息发布核心逻辑
//!
//! 通过 msg-api-client 的 Feign 客户端调用 msg-svr 完成模板查询、渲染和投递创建。

use crate::config::msg_pub_config;
use msg_api_client::dto::MsgDeliveryAddDto;
use msg_api_client::dto::MsgMessageQueryDto;
use msg_api_client::MsgDeliveryApiClient;
use msg_api_client::MsgMessageApiClient;
use robotech::api::U64;
use robotech::api_client::ApiClientConfig;
use robotech::micro_svc::FeignApiClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// 消息发送的请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishRequest {
    /// 事件编码，对应 `msg_message.event_code`
    pub event_code: String,
    /// 模板参数，key 对应模板中的 `{key}` 占位符
    ///
    /// 例如模板 `订单 {order_id} 已发货`，传入 `{"order_id": "12345"}`
    pub params: HashMap<String, String>,
    /// 业务 ID，幂等去重用。同一 business_id 重复调用不会产生多条投递
    pub business_id: i64,
}

/// 消息发送响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    /// 投递记录 ID，可通过此 ID 查询投递状态
    pub delivery_id: i64,
}

/// 发送一条消息
///
/// 根据 `event_code` 查找消息模板，用 `params` 渲染标题和内容，
/// 调用 msg-svr 创建投递记录并触发发送。
pub async fn publish(
    event_code: &str,
    params: HashMap<String, String>,
    business_id: i64,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let config = msg_pub_config::get()?;

    let base_url = config
        .msg_svr_base_url
        .as_ref()
        .ok_or("msg_svr_base_url 未配置，无法发送消息")?;

    // 查询消息模板
    let message = fetch_message_by_event_code(base_url, event_code).await?;

    debug!(
        "找到消息模板: id={}, name={}, event_code={}",
        message.id, message.name, message.event_code
    );

    let title = message
        .title_template
        .as_deref()
        .map(|t| render_template(t, &params))
        .unwrap_or_default();
    let content = message
        .content_template
        .as_deref()
        .map(|t| render_template(t, &params))
        .unwrap_or_default();

    debug!(
        "模板渲染完成: title={}",
        &title[..title.len().min(50)]
    );

    // 创建投递记录
    let delivery = create_delivery(
        base_url,
        &message,
        business_id,
        &title,
        &content,
    )
    .await?;

    info!(
        "消息发送成功: delivery_id={}, event_code={}, business_id={}",
        delivery.id, event_code, business_id
    );

    Ok(delivery.id)
}

/// 发送消息（带重试）
pub async fn publish_with_retry(
    event_code: &str,
    params: HashMap<String, String>,
    business_id: i64,
) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
    let config = msg_pub_config::get()?;
    let retry_count = config.publish_retry_count;
    let retry_interval = config.publish_retry_interval;

    let mut last_error = None;
    for attempt in 0..=retry_count {
        match publish(event_code, params.clone(), business_id).await {
            Ok(delivery_id) => return Ok(delivery_id),
            Err(e) => {
                warn!("发送失败 (attempt {}/{}): {}", attempt + 1, retry_count + 1, e);
                last_error = Some(e);
                if attempt < retry_count {
                    tokio::time::sleep(retry_interval).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "发送失败，已达最大重试次数".into()))
}

// ====================================================================
//  内部实现
// ====================================================================

/// 创建 Simple 模式 Feign 客户端
async fn build_feign_client(base_url: &str) -> FeignApiClient {
    FeignApiClient::new(ApiClientConfig::Simple {
        base_url: base_url.to_string(),
        auth: None,
    })
    .await
}

/// 通过 feign 客户端查询消息模板
async fn fetch_message_by_event_code(
    base_url: &str,
    event_code: &str,
) -> Result<msg_api_client::vo::MsgMessageVo, Box<dyn std::error::Error + Send + Sync>> {
    let client = build_feign_client(base_url).await;
    let api = MsgMessageApiClient::new(client);

    let query = MsgMessageQueryDto::builder()
        .event_code(event_code.to_string())
        .build();

    let ro = api
        .list_by_query_dto(&query, 0)
        .await
        .map_err(|e| format!("查询消息模板失败: {e}"))?;

    if !ro.is_ok() {
        return Err(format!("查询消息模板失败: {}", ro.msg).into());
    }

    let messages = ro.extra.unwrap_or_default();
    messages
        .into_iter()
        .next()
        .ok_or_else(|| format!("未找到事件编码为 '{}' 的消息模板", event_code).into())
}

/// 模板渲染：将 `{key}` 占位符替换为 `params` 中的值
fn render_template(template: &str, params: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in params {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    result
}

/// 通过 feign 客户端创建投递记录
async fn create_delivery(
    base_url: &str,
    message: &msg_api_client::vo::MsgMessageVo,
    business_id: i64,
    title: &str,
    content: &str,
) -> Result<msg_api_client::vo::MsgDeliveryVo, Box<dyn std::error::Error + Send + Sync>> {
    let client = build_feign_client(base_url).await;
    let api = MsgDeliveryApiClient::new(client);

    let now_ms = chrono::Utc::now().timestamp_millis();

    let add_dto = MsgDeliveryAddDto::builder()
        .event_code(message.event_code.clone())
        .message_id(message.id)
        .event_source_id(message.event_source_id)
        .message_category_id(message.category_id)
        .business_id(business_id)
        .business_trigger_ms(now_ms)
        .title(title.to_string())
        .content(content.to_string())
        ._current_user_id(U64(0))
        .build();

    let ro = api
        .add(&add_dto)
        .await
        .map_err(|e| format!("创建投递记录失败: {e}"))?;

    if !ro.is_ok() {
        let detail = ro.detail.as_deref().unwrap_or(&ro.msg);
        error!("创建投递记录失败: {}", detail);
        return Err(format!("创建投递记录失败: {}", detail).into());
    }

    ro.extra
        .ok_or_else(|| "创建投递记录响应缺少数据".into())
}