//! # 消息处理
//!
//! 从 NATS 收到消息后，解析负载、匹配消息模板、渲染内容、
//! 创建投递记录及关联的投递目标和投递渠道。

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

/// 匹配 `{variable_name}` 与 `{variable_name:json}` 两种占位符
static TEMPLATE_VAR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{(\w+)(?::(json))?}").expect("模板正则表达式编译失败")
});

/// 渲染模板字符串，把 `{key}` / `{key:json}` 替换成消息里的变量值
///
/// 变量取自消息的 `labels` 与 `annotations`：两者都是发布方给的 key-value，
/// 模板只认键名，不关心它来自哪一份。同名时以 `labels` 为准——`labels` 是
/// 发布方声明的结构化事实，`annotations` 里的同名键通常是它的可读副本。
///
/// 渲染完就只剩标题、正文与附加内容，`labels` / `annotations` 不再随投递
/// 下发：它们是模板的输入，投递的产物是渲染结果。
///
/// # 两种占位符
///
/// - `{key}`：原样代入，用于正文，或 JSON 里的数字/布尔。
/// - `{key:json}`：代入 JSON 字符串字面量（自带引号并转义），用于 JSON 里的
///   字符串值。**模板里不要再包一层引号**：
///   `{"text":{deviceName:json}}` 渲染成 `{"text":"1F 东区"}`；
///   写成 `"{"deviceName:json}"` 就成了 `""1F 东区""`，不是合法 JSON。
///
/// `{key}` 不做任何转义，所以**不要把它放进 JSON 字符串内部**：
/// `{"text":"{deviceName}"}` 在设备名带引号时会把 JSON 撑破。字符串值一律
/// 用 `{key:json}` 整体代入；要在字符串中间拼变量，就把它拆成独立的键。
///
/// 缺失的变量：`{key:json}` 输出 `null`（保证 JSON 仍然合法），`{key}` 保留
/// 原占位符并 warn（人看的正文里能一眼看出来）。
pub(crate) fn render_template(
    template: &str,
    labels: &HashMap<String, String>,
    annotations: &HashMap<String, String>,
) -> String {
    TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            let as_json = caps.get(2).is_some();
            match labels.get(key).or_else(|| annotations.get(key)) {
                Some(v) if as_json => {
                    // 值本身带引号、反斜杠、换行时，不转义就会把外层的 JSON 撑破
                    serde_json::to_string(v).unwrap_or_else(|_| "null".to_owned())
                }
                Some(v) => v.clone(),
                None if as_json => {
                    warn!("模板变量未提供: {{{key}:json}}，输出 null");
                    "null".to_owned()
                }
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

    /// 值里带引号时，`:json` 必须转义，否则整段 JSON 被撑破
    #[test]
    fn test_render_template_json_escapes_quotes() {
        let labels = vars(&[("deviceName", "1F \"东\" 区烟感")]);
        let template = r#"{"text":{deviceName:json}}"#;
        let result = render_template(template, &labels, &HashMap::new());
        assert_eq!(result, r#"{"text":"1F \"东\" 区烟感"}"#);
        // 关键断言：渲染结果本身是合法 JSON
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert_eq!(parsed["text"], "1F \"东\" 区烟感");
    }

    /// 数字不加引号：`:json` 只给字符串用，数值占位符保持裸代入
    #[test]
    fn test_render_template_json_with_numeric_placeholder() {
        let annotations = vars(&[("lon", "113.9"), ("lat", "22.5")]);
        let template = r#"{"lon":{lon},"lat":{lat}}"#;
        let result = render_template(template, &HashMap::new(), &annotations);
        assert_eq!(result, r#"{"lon":113.9,"lat":22.5}"#);
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert_eq!(parsed["lon"], 113.9);
    }

    /// 缺变量时 `:json` 输出 null，JSON 结构还在；裸占位符保留原样
    #[test]
    fn test_render_template_json_missing_variable_is_null() {
        let labels = vars(&[("locatorCode", "L001")]);
        let template = r#"{"code":{locatorCode:json},"level":{floor:json}}"#;
        let result = render_template(template, &labels, &HashMap::new());
        assert_eq!(result, r#"{"code":"L001","level":null}"#);
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert!(parsed["level"].is_null());
    }
}
