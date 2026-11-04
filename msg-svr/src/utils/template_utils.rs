//! # 消息模板渲染（适配层）
//!
//! 语法、过滤器与错误语义都在框架层 [`robotech::template`] 里；本模块只做
//! 三件 msg 特有的事：
//!
//! 1. 组装变量优先级：内置变量（`_` 开头）> `labels` > `annotations`；
//! 2. 从 `[msg] template-time-offset` 取模板时区；
//! 3. 把 crate 返回的结构化诊断落成日志。
//!
//! ## 多引擎支持
//!
//! 模板首行可以用 `@engine: <name>` 声明使用哪个引擎：
//!
//! ```text
//! @engine: robotech       使用内置引擎（默认，向后兼容）
//! @engine: minijinja      使用 minijinja 引擎（支持 if/for/switch 等控制流）
//! ```
//!
//! 没有声明时默认使用 `robotech` 引擎。
//!
//! ## robotech 引擎占位符语法
//!
//! ```text
//! {{key}}                         原样代入
//! {{key:过滤器}}                   一个过滤器
//! {{key:过滤器(参数)}}              带参数的过滤器
//! {{key:过滤器(参数)|过滤器}}       用 `|` 串成链，从左到右执行
//! ```
//!
//! `${key}` / `${key:过滤器}` 是与 `{{...}}` **完全等价**的别名（同一套过滤器与
//! 缺值语义），可同模板混用；`$` 不做环境变量解释。
//!
//! **旧语法 `{key}` 已废弃，不再支持。** 库里已有的旧模板由迁移
//! `20261103000000_switch-template-syntax-to-double-braces.sql` 一次性转换；
//! 新模板必须用 `{{...}}`。
//!
//! 参数用括号包住，所以参数里可以有 `|`、`:`、空格（`{{url:default(a|b)}}`）。
//! 参数到**第一个** `)` 为止，需要 `)` 本身时目前无法表达——内置过滤器都用不上。
//!
//! ## minijinja 引擎语法
//!
//! minijinja 引擎支持 Jinja2 风格的控制流：
//!
//! ```text
//! @engine: minijinja
//! {% if status == "critical" %}
//! 🚨 严重告警：{{name}}
//! {% else %}
//! ℹ️ 通知：{{name}}
//! {% endif %}
//! ```
//!
//! ## 内置过滤器（robotech 引擎）
//!
//! `json` / `time` / `since` / `upper` / `lower` / `trim` / `default` /
//! `truncate` / `fixed` / `mask` / `url`，语义见 [`robotech::template`] 的文档。
//!
//! ## 内置变量
//!
//! `_` 开头的是服务端注入的变量，优先级高于 `labels` / `annotations`
//! （发布方写同名键会被忽略，`_` 前缀是保留命名空间）：
//!
//! | 变量 | 含义 |
//! |---|---|
//! | `_create_ms` | 这条消息的处理时间（毫秒） |
//! | `_business_trigger_ms` | 业务触发时间（毫秒），发布方没给时不存在 |
//! | `_business_id` | 业务 ID，发布方没给时不存在 |
//! | `_message_id` | 消息模板 ID |
//! | `_event_code` | 事件编码 |
//!
//! 这样发布方不必每次往 `labels` 里塞时间戳，`{{_create_ms:time}}` 直接可用。
//!
//! ## 时间戳为什么需要显式 `time`
//!
//! 变量都是字符串，引擎分不清 `1735689600000` 是时间戳还是普通数字，所以
//! 格式化必须由模板声明。时区取 `[msg] template-time-offset`（默认 `+08:00`），
//! 与进程所在机器的 `TZ` 无关——容器里通常是 UTC，靠环境变量才能对上本地
//! 时间太脆。

use crate::config::get_msg_config;
use chrono::Utc;
use robotech::svc::SvcError;
use robotech::template::{
    FilterContext, PriorityVariables, Severity,
    render as render_unified, validate as validate_unified,
};
use std::collections::HashMap;
use tracing::warn;
use validator::ValidationError;

/// # 渲染模板字符串
///
/// 变量优先级：内置变量（`_` 开头，服务端注入）> `labels` > `annotations`。
///
/// `labels` 与 `annotations` 都是发布方给的 key-value，模板只认键名，不关心
/// 它来自哪一份。同名时以 `labels` 为准——`labels` 是发布方声明的结构化事实，
/// `annotations` 里的同名键通常是它的可读副本。
///
/// 渲染完就只剩标题、正文与附加内容，`labels` / `annotations` 不再随投递
/// 下发：它们是模板的输入，投递的产物是渲染结果。
///
/// ## 用法
///
/// ```text
/// 你好 {{name}}，设备 {{deviceName:json}} 于 {{_create_ms:time(%m-%d %H:%M)}} 报警
/// {"text":{{deviceName:json}},"time":{{triggerMs:time(%F %T)|json}}}
/// 链接 https://x/alarm?code={{locatorCode:url}}
/// 最后上报 {{lastSeenMs:since}}
/// ```
pub(crate) fn render_template(
    template: &str,
    builtins: &HashMap<&'static str, String>,
    labels: &HashMap<String, String>,
    annotations: &HashMap<String, String>,
) -> String {
    let vars = PriorityVariables::new()
        .layer(builtins)
        .layer(labels)
        .layer(annotations);

    // 每条消息都读一次：配置热更新后下一次渲染即生效，不用重启
    let ctx = FilterContext::new(
        get_msg_config().effective_time_offset(),
        Utc::now().timestamp_millis(),
    );

    let outcome = render_unified(template, &vars, &ctx);
    for diagnostic in &outcome.diagnostics {
        // 诊断的严重级别由调用方决定：消息中心按 warn 处理，不阻塞投递
        warn!("{diagnostic}");
    }
    outcome.text
}

/// # 保存前的模板校验
///
/// 拦截 Error 级问题：`{{` / `${` 未闭合、残留旧语法 `{key}`、过滤器语法错误、
/// 未知过滤器。`${key}` 是合法别名，不会被当成旧语法或未知写法。
/// 渲染期的 Warning（变量缺失、单个过滤器失败）不拦——变量是否存在只有发布时
/// 才知道，保存阶段无法判断。
///
/// 校验只在模板非空时执行；`None` 表示「用发布方给的值」，不涉及模板语法。
pub(crate) fn validate_template_field(
    field: &str,
    template: Option<&str>,
) -> Result<(), SvcError> {
    let Some(template) = template else {
        return Ok(());
    };

    let check = validate_unified(template);
    if !check.has_errors() {
        return Ok(());
    }

    let reasons: Vec<String> = check
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .map(ToString::to_string)
        .collect();

    Err(SvcError::Validation(
        ValidationError::new("模板语法错误").with_message(
            format!("{field}: {}", reasons.join("；")).into(),
        ),
    ))
}
