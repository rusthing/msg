//! # 消息处理
//!
//! 从 NATS 收到消息后，解析负载、匹配消息模板、渲染内容、
//! 创建投递记录及关联的投递目标和投递渠道。

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

/// 匹配 `{variable_name}` 形式的占位符
static TEMPLATE_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)}").expect("模板正则表达式编译失败"));

/// 渲染模板字符串，把 `{key}` 替换成消息里的变量值
///
/// 变量取自消息的 `labels` 与 `annotations`：两者都是发布方给的 key-value，
/// 模板只认键名，不关心它来自哪一份。同名时以 `labels` 为准——`labels` 是
/// 发布方声明的结构化事实，`annotations` 里的同名键通常是它的可读副本。
///
/// 渲染完就只剩标题和正文，`labels` / `annotations` 不再随投递下发：
/// 它们是模板的输入，投递的产物是渲染结果。
///
/// 未提供的 key 保留原占位符并 warn。
pub(crate) fn render_template(
    template: &str,
    labels: &HashMap<String, String>,
    annotations: &HashMap<String, String>,
) -> String {
    TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            match labels.get(key).or_else(|| annotations.get(key)) {
                Some(v) => v.clone(),
                None => {
                    warn!("模板变量未提供: {{{key}}}，保留占位符");
                    caps[0].to_owned()
                }
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn test_render_template_with_all_variables() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let labels = vars(&[("name", "张三"), ("order_id", "ORD-001")]);
        let result = render_template(template, &labels, &HashMap::new());
        assert_eq!(result, "你好 张三，你的订单 ORD-001 已发货");
    }

    #[test]
    fn test_render_template_with_missing_variable() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let result = render_template(template, &HashMap::new(), &HashMap::new());
        assert_eq!(result, "你好 {name}，你的订单 {order_id} 已发货");
    }

    #[test]
    fn test_render_template_with_partial_variables() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let labels = vars(&[("name", "李四")]);
        let result = render_template(template, &labels, &HashMap::new());
        assert_eq!(result, "你好 李四，你的订单 {order_id} 已发货");
    }

    #[test]
    fn test_render_template_no_placeholders() {
        let template = "系统通知：服务已重启";
        let result = render_template(template, &HashMap::new(), &HashMap::new());
        assert_eq!(result, "系统通知：服务已重启");
    }

    #[test]
    fn test_render_template_variables_from_annotations() {
        // 定位消息的坐标只在 annotations 里，模板照样能取到
        let template = "定位卡 {locatorCode} 位于 {lat},{lon}";
        let labels = vars(&[("locatorCode", "L001")]);
        let annotations = vars(&[("lat", "24.4659"), ("lon", "109.7192")]);
        let result = render_template(template, &labels, &annotations);
        assert_eq!(result, "定位卡 L001 位于 24.4659,109.7192");
    }

    #[test]
    fn test_render_template_labels_win_on_conflict() {
        let labels = vars(&[("locatorCode", "L001")]);
        let annotations = vars(&[("locatorCode", "L999")]);
        let result = render_template("{locatorCode}", &labels, &annotations);
        assert_eq!(result, "L001");
    }
}
