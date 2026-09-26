//! # 消息处理
//!
//! 从 NATS 收到消息后，解析负载、匹配消息模板、渲染内容、
//! 创建投递记录及关联的投递目标和投递渠道。

use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::{error, info, warn};

use crate::cache::{get_msg_cache, CachedMessage};
use crate::dto::{
    MsgDeliveryAddDto, MsgDeliveryChannelAddDto, MsgDeliveryQueryDto,
    MsgDeliveryTargetAddDto,
};
use crate::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc};
use idworker::next_id;
use robotech::api::U64;
use robotech::db::get_db_conn;
use robotech::mq::nats::NatsError;

/// 从 NATS 收到的消息负载
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingMessage {
    pub event_code: String,
    pub business_id: i64,
    pub business_trigger_ms: Option<i64>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

/// 匹配 `{variable_name}` 形式的占位符
static TEMPLATE_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)\}").expect("模板正则表达式编译失败"));

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
pub async fn process_message(
    subject: &str,
    payload: &[u8],
) -> Result<(), NatsError> {
    // ── 1. 解析消息负载 ──

    let incoming: IncomingMessage = serde_json::from_slice(payload).map_err(|e| {
        NatsError::Handle(format!(
            "消息负载 JSON 解析失败 [{}]: {}",
            subject, e
        ))
    })?;

    info!(
        "收到消息: event_code={}, business_id={}, subject={}",
        incoming.event_code, incoming.business_id, subject
    );

    // ── 2. 查找消息模板 ──

    let cache = get_msg_cache();
    let msg: &CachedMessage = cache
        .messages
        .get(&incoming.event_code)
        .ok_or_else(|| {
            NatsError::Handle(format!(
                "未找到事件编码对应的消息模板: {}",
                incoming.event_code
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
        Some(tpl) => render_template(tpl, &incoming.labels),
        None => incoming.annotations.get("title").cloned().unwrap_or_default(),
    };

    let content = match &msg.content_template {
        Some(tpl) => render_template(tpl, &incoming.labels),
        None => incoming
            .annotations
            .get("content")
            .cloned()
            .unwrap_or_default(),
    };

    if title.is_empty() && content.is_empty() {
        return Err(NatsError::Handle(
            "标题和内容均为空，无法投递".into(),
        ));
    }

    info!("内容渲染完成: title={title}");

    // ── 5. 幂等检查 ──

    let db = get_db_conn().map_err(|e| NatsError::Handle(e.to_string()))?;

    let existing = MsgDeliverySvc::get_by_query_dto(
        MsgDeliveryQueryDto {
            business_id: Some(incoming.business_id),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await;

    match existing {
        Ok(ro) if ro.extra.is_some() => {
            info!(
                "消息已投递，幂等跳过: business_id={}",
                incoming.business_id
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
    let delivery_id = next_id()
        .map_err(|e| NatsError::Handle(format!("生成投递ID失败: {e}")))?;

    let labels_json = if incoming.labels.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&incoming.labels).map_err(|e| {
            NatsError::Handle(format!("labels 序列化失败: {e}"))
        })?)
    };

    let annotations_json = if incoming.annotations.is_empty() {
        None
    } else {
        Some(serde_json::to_string(&incoming.annotations).map_err(|e| {
            NatsError::Handle(format!("annotations 序列化失败: {e}"))
        })?)
    };

    let delivery_add = MsgDeliveryAddDto {
        id: Some(U64(delivery_id)),
        event_code: Some(msg.event_code.clone()),
        message_id: Some(msg.id),
        event_source_id: Some(msg.source.as_ref().map(|s| s.id).unwrap_or(0)),
        message_category_id: Some(msg.category.as_ref().map(|c| c.id).unwrap_or(0)),
        business_id: Some(incoming.business_id),
        business_trigger_ms: Some(incoming.business_trigger_ms.unwrap_or_else(|| now_ms as i64)),
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
                incoming.business_id
            ))
        })?;

    info!("投递记录已创建: delivery_id={delivery_id}, message_id={}", msg.id);

    // ── 7. 创建投递目标 ──

    for target in &msg.targets {
        let target_category_id = match cache.target_categories.get(&target.target_category_code) {
            Some(id) => *id,
            None => {
                error!(
                    "未找到目标类别ID: code={}, 跳过此目标",
                    target.target_category_code
                );
                continue;
            }
        };

        let target_id_val = next_id()
            .map_err(|e| NatsError::Handle(format!("生成投递目标ID失败: {e}")))?;

        let target_add = MsgDeliveryTargetAddDto {
            id: Some(U64(target_id_val)),
            delivery_id: Some(delivery_id as i64),
            target_category_id: Some(target_category_id),
            target_id: Some(target.target_id),
            deliver_target_status: Some(DeliverTargetStatus::Delivering),
            _current_ms: Some(U64(now_ms)),
            _current_user_id: U64(0),
        };

        match MsgDeliveryTargetSvc::add(target_add, Some(db.as_ref())).await {
            Ok(ro) => {
                let created_target = ro.extra.as_ref().unwrap();
                info!(
                    "投递目标已创建: target_id={}, category={}, delivery_id={delivery_id}",
                    created_target.id, target.target_category_code
                );

                // ── 8. 为每个渠道创建投递渠道记录 ──

                for channel in &msg.channels {
                    let channel_id_val = next_id().map_err(|e| {
                        NatsError::Handle(format!("生成投递渠道ID失败: {e}"))
                    })?;

                    let channel_add = MsgDeliveryChannelAddDto {
                        id: Some(U64(channel_id_val)),
                        deliver_target_id: Some(created_target.id),
                        channel_id: Some(channel.id),
                        deliver_channel_status: Some(DeliverChannelStatus::Delivering),
                        address: None,
                        _current_ms: Some(U64(now_ms)),
                        _current_user_id: U64(0),
                    };

                    if let Err(e) =
                        MsgDeliveryChannelSvc::add(channel_add, Some(db.as_ref())).await
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
                    "创建投递目标失败: category={}, error={e}",
                    target.target_category_code
                );
            }
        }
    }

    info!(
        "消息处理完成: event_code={}, business_id={}",
        incoming.event_code, incoming.business_id
    );

    Ok(())
}

/// 渲染模板字符串，将 `{key}` 替换为 `labels` 中对应的值
///
/// 未提供的 key 保留原占位符并 warn。
fn render_template(template: &str, labels: &HashMap<String, String>) -> String {
    TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            match labels.get(key) {
                Some(v) => v.clone(),
                None => {
                    warn!("模板变量 labels 未提供: {{{key}}}，保留占位符");
                    caps[0].to_owned()
                }
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_template_with_all_variables() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let mut labels = HashMap::new();
        labels.insert("name".to_string(), "张三".to_string());
        labels.insert("order_id".to_string(), "ORD-001".to_string());
        let result = render_template(template, &labels);
        assert_eq!(result, "你好 张三，你的订单 ORD-001 已发货");
    }

    #[test]
    fn test_render_template_with_missing_variable() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let labels = HashMap::new();
        let result = render_template(template, &labels);
        assert_eq!(result, "你好 {name}，你的订单 {order_id} 已发货");
    }

    #[test]
    fn test_render_template_with_partial_variables() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let mut labels = HashMap::new();
        labels.insert("name".to_string(), "李四".to_string());
        let result = render_template(template, &labels);
        assert_eq!(result, "你好 李四，你的订单 {order_id} 已发货");
    }

    #[test]
    fn test_render_template_no_placeholders() {
        let template = "系统通知：服务已重启";
        let labels = HashMap::new();
        let result = render_template(template, &labels);
        assert_eq!(result, "系统通知：服务已重启");
    }
}