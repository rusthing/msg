//! # 消息处理
//!
//! 从 NATS 收到消息后，解析负载、匹配消息模板、渲染内容、
//! 创建投递记录及关联的投递目标和投递渠道。

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::{error, info, warn};

use crate::cache::{get_msg_cache, MessageCache};
use crate::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use crate::dto::{
    MsgDeliveryAddDto, MsgDeliveryChannelAddDto, MsgDeliveryQueryDto, MsgDeliveryTargetAddDto,
};
use crate::svc::{MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc};
use idworker::next_id;
use msg_api::mqo::message_mqo::MessageMqo;
use robotech::api::U64;
use robotech::db::get_db_conn;
use robotech::mq::nats::NatsError;

/// 匹配 `{variable_name}` 形式的占位符
static TEMPLATE_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)}").expect("模板正则表达式编译失败"));

/// 渲染模板字符串，将 `{key}` 替换为 `labels` 中对应的值
///
/// 未提供的 key 保留原占位符并 warn。
pub(crate) fn render_template(template: &str, annotations: &HashMap<String, String>) -> String {
    TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            match annotations.get(key) {
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
