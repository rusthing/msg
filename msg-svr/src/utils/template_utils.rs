//! # 消息模板渲染
//!
//! 把 `msg_message.title_template` / `content_template` / `ext_template` 里的
//! 占位符替换成变量值，并按需做格式化。
//!
//! ## 占位符语法
//!
//! ```text
//! {key}                          原样代入
//! {key:过滤器}                    一个过滤器
//! {key:过滤器(参数)}               带参数的过滤器
//! {key:过滤器(参数)|过滤器}        用 `|` 串成链，从左到右执行
//! ```
//!
//! 参数用括号包住，所以参数里可以有 `|`、`:`、空格（`{url:default(a|b)}`）。
//! 参数到**第一个** `)` 为止，需要 `)` 本身时目前无法表达——内置过滤器都用不上。
//!
//! ## 内置过滤器
//!
//! | 过滤器 | 参数 | 作用 |
//! |---|---|---|
//! | `json` | — | 转成 JSON 字符串字面量（自带引号并转义） |
//! | `time` | strftime 格式，缺省 `%Y-%m-%d %H:%M:%S` | 时间戳 → 可读时间 |
//! | `since` | — | 时间戳 → 相对时间（`3 分钟前`） |
//! | `upper` / `lower` | — | 大小写转换 |
//! | `trim` | — | 去首尾空白 |
//! | `default` | 兜底文本 | 变量缺失或值为空白时替换成兜底文本 |
//! | `truncate` | 字符数 | 按**字符**截断（中文不会被切半个），超长时补 `…` |
//! | `fixed` | 小数位 | 数字按固定小数位输出，非数字原样 |
//! | `mask` | `首,尾` 位数 | 脱敏：保留首尾若干字符，中间换成 `*` |
//! | `url` | — | percent-encode（按查询参数值的规则，空格编成 `%20`） |
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
//! 这样发布方不必每次往 `labels` 里塞时间戳，`{_create_ms:time}` 直接可用。
//!
//! ## 扩一个新过滤器
//!
//! 1. 写一个 [`FilterFn`] 签名的函数；
//! 2. 在 [`FILTERS`] 表里加一行 `("名字", 函数名)`；
//! 3. 在文件末尾的测试里加用例。
//!
//! 没有第四步：解析、链式执行、错误降级都是共用的。
//!
//! ## 时间戳为什么需要显式 `time`
//!
//! 变量都是字符串，引擎分不清 `1735689600000` 是时间戳还是普通数字，所以
//! 格式化必须由模板声明。时区取 `[msg] template-time-offset`（默认 `+08:00`），
//! 与进程所在机器的 `TZ` 无关——容器里通常是 UTC，靠环境变量才能对上本地
//! 时间太脆。

use crate::config::get_msg_config;
use chrono::{FixedOffset, TimeZone, Utc};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;
use tracing::warn;

/// `time` 不带格式串时使用的默认格式
const DEFAULT_TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

/// 秒级与毫秒级时间戳的分界
///
/// `1e11` 毫秒是 1973 年，`1e11` 秒是公元 5138 年——两个量级不会撞车，
/// 小于这个数的一律当秒。发布方把秒级时间戳塞进 `labels` 是常见手滑。
const SECONDS_TS_UPPER_BOUND: u64 = 100_000_000_000;

/// 匹配 `{key}` 与 `{key:<过滤器链>}`，过滤器链里不允许出现花括号
///
/// 宽松匹配（`[^{}]*`）而不是只认已知过滤器，是为了让写错的过滤器能被识别
/// 出来并告警，而不是悄悄留成原文。模板里的 JSON 花括号（`{"a":1}`）因为
/// `{` 后面不是 `\w`，不会被误当成占位符。
static TEMPLATE_VAR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\{(\w+)(?::([^{}]*))?}").expect("模板正则表达式编译失败"));

/// 过滤器执行上下文
///
/// 目前只有时区。以后需要「当前时间」「字典表」这类外部输入时加在这里，
/// 过滤器签名不用动。
struct FilterCtx {
    /// 模板时区偏移（`time` 用）
    time_offset: FixedOffset,
}

/// 过滤器函数
///
/// ## 参数
///
/// * `value` - 当前值。`None` 表示变量缺失，或前序过滤器判定为「无值」
/// * `arg` - `名字(参数)` 里的参数，无参数时为 `None`
/// * `ctx` - 执行上下文
///
/// ## 返回值
///
/// * `Ok(Some(v))` - 正常处理，`v` 交给下一个过滤器
/// * `Ok(None)` - 仍然无值，引擎最终按「变量缺失」处理
/// * `Err(原因)` - 这个过滤器做不了（参数写错、值格式不对）。引擎 warn 后
///   **跳过它**、保留当前值继续往后走：一个字段格式不对不该让整条消息投不出去
type FilterFn = fn(Option<&str>, Option<&str>, &FilterCtx) -> Result<Option<String>, String>;

/// 过滤器注册表：模板里 `{key:名字}` 的「名字」在这里查
///
/// 加过滤器只需在这里加一行。未知名字会被引擎当成模板写错处理——保留原
/// 占位符并 warn，不做「猜你想用哪个」的模糊匹配。
static FILTERS: LazyLock<HashMap<&'static str, FilterFn>> = LazyLock::new(|| {
    HashMap::from([
        ("json", filter_json as FilterFn),
        ("time", filter_time),
        ("since", filter_since),
        ("upper", filter_upper),
        ("lower", filter_lower),
        ("trim", filter_trim),
        ("default", filter_default),
        ("truncate", filter_truncate),
        ("fixed", filter_fixed),
        ("mask", filter_mask),
        ("url", filter_url),
    ])
});

/// 一次过滤器调用：名字 + 可选参数
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FilterCall<'a> {
    /// 过滤器名，在 [`FILTERS`] 里查
    name: &'a str,
    /// 括号里的参数，原样保留（不 trim，各过滤器自己决定怎么解释）
    arg: Option<&'a str>,
}

/// # 解析过滤器链
///
/// 语法：`名字` 或 `名字(参数)`，多项用 `|` 连接。手写扫描而不是 `split('|')`
/// ——参数在括号里，可以包含 `|`，全局切分会把参数切断。
///
/// ## 参数
///
/// * `spec` - `{key:` 之后、`}` 之前的部分，空串表示没有过滤器
///
/// ## 返回值
///
/// 解析成功返回调用列表；语法错误返回带位置的说明，调用方据此保留原占位符。
fn parse_filters(spec: &str) -> Result<Vec<FilterCall<'_>>, String> {
    let mut calls = Vec::new();
    let bytes = spec.as_bytes();
    let mut cursor = 0;

    loop {
        // 允许 `a | b` 这种写法，分隔符周围留空不影响
        while cursor < bytes.len() && (bytes[cursor] == b'|' || bytes[cursor].is_ascii_whitespace())
        {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let name_start = cursor;
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric()
                || bytes[cursor] == b'_'
                || bytes[cursor] == b'-')
        {
            cursor += 1;
        }
        if name_start == cursor {
            return Err(format!("{} 处缺少过滤器名", &spec[name_start..]));
        }
        let name = &spec[name_start..cursor];

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let arg = if cursor < bytes.len() && bytes[cursor] == b'(' {
            let arg_start = cursor + 1;
            let Some(close) = spec[arg_start..].find(')') else {
                return Err(format!("{name} 的参数缺少右括号"));
            };
            cursor = arg_start + close + 1;
            Some(&spec[arg_start..cursor - 1])
        } else {
            None
        };

        calls.push(FilterCall { name, arg });

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor < bytes.len() && bytes[cursor] != b'|' {
            return Err(format!(
                "{name} 之后有多余字符：{}",
                spec[cursor..].trim_end_matches('}')
            ));
        }
    }

    Ok(calls)
}

/// 断言过滤器不带参数
fn no_arg(arg: Option<&str>, name: &str) -> Result<(), String> {
    match arg {
        None => Ok(()),
        Some(arg) if arg.trim().is_empty() => Ok(()),
        Some(arg) => Err(format!("{name} 不接受参数（给了 {arg}）")),
    }
}

/// 解析参数里的非负整数
fn parse_arg_usize(arg: Option<&str>, hint: &str) -> Result<usize, String> {
    let arg = arg
        .filter(|a| !a.trim().is_empty())
        .ok_or_else(|| hint.to_owned())?;
    arg.trim()
        .parse::<usize>()
        .map_err(|_| format!("参数 {arg} 不是非负整数"))
}

/// # 解析时间戳字符串为毫秒
///
/// 秒级 / 毫秒级自动识别（见 [`SECONDS_TS_UPPER_BOUND`]），并挡住超出
/// `i64` 表示范围的值——`as i64` 静默回绕会变成 1970 年前。
fn parse_ts_ms(value: &str) -> Result<i64, String> {
    let raw = value
        .trim()
        .parse::<u64>()
        .map_err(|_| format!("{value} 不是时间戳"))?;
    let ms = if raw < SECONDS_TS_UPPER_BOUND {
        raw.saturating_mul(1000)
    } else {
        raw
    };
    if ms > i64::MAX as u64 {
        return Err(format!("{value} 超出可表示范围"));
    }
    Ok(ms as i64)
}

/// `json`：把值转成 JSON 字符串字面量
///
/// 缺失时输出 `null` 而不是保持缺失——`ext` 模板是 JSON，结构不能塌。
fn filter_json(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "json")?;
    Ok(Some(match value {
        // 值本身带引号、反斜杠、换行时，不转义就会把外层的 JSON 撑破
        Some(v) => serde_json::to_string(v).unwrap_or_else(|_| "null".to_owned()),
        None => "null".to_owned(),
    }))
}

/// `time`：时间戳 → 可读时间
///
/// 秒级 / 毫秒级自动识别，按 `ctx.time_offset` 换算时区。
fn filter_time(
    value: Option<&str>,
    arg: Option<&str>,
    ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let fmt = arg.filter(|f| !f.is_empty()).unwrap_or(DEFAULT_TIME_FORMAT);
    let ms = parse_ts_ms(value)?;

    Utc.timestamp_millis_opt(ms)
        .single()
        .map(|dt| Some(dt.with_timezone(&ctx.time_offset).format(fmt).to_string()))
        .ok_or_else(|| format!("{value} 超出可表示范围"))
}

/// # 把毫秒差描述成相对时间
///
/// 告警文案里 `3 分钟前` 比时间戳直观。未来时间（时钟偏差）一律按「刚刚」，
/// 免得出现「-1 分钟前」这种让人以为系统坏了的输出。
fn humanize_gap(diff_ms: i64) -> String {
    const MINUTE: i64 = 60_000;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;

    if diff_ms < MINUTE {
        return "刚刚".to_owned();
    }
    if diff_ms < HOUR {
        return format!("{} 分钟前", diff_ms / MINUTE);
    }
    if diff_ms < DAY {
        return format!("{} 小时前", diff_ms / HOUR);
    }
    if diff_ms < 30 * DAY {
        return format!("{} 天前", diff_ms / DAY);
    }
    if diff_ms < 365 * DAY {
        return format!("{} 个月前", diff_ms / (30 * DAY));
    }
    format!("{} 年前", diff_ms / (365 * DAY))
}

/// `since`：时间戳 → 相对时间
///
/// 相对时间是**渲染时刻**算出来的，同一条消息晚一点重放会变成另一个字符串。
/// 需要可追溯的快照就别用它：`content` 里用 `time`，`ext` 里把原始时间戳
/// 裸代入（`{_create_ms}` / `{_business_trigger_ms}`）留给消费方自己算差。
fn filter_since(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "since")?;
    let Some(value) = value else {
        return Ok(None);
    };
    let ms = parse_ts_ms(value)?;
    let now = Utc::now().timestamp_millis();
    Ok(Some(humanize_gap(now.saturating_sub(ms))))
}

/// `upper`：转大写
fn filter_upper(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "upper")?;
    Ok(value.map(str::to_uppercase))
}

/// `lower`：转小写
fn filter_lower(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "lower")?;
    Ok(value.map(str::to_lowercase))
}

/// `trim`：去首尾空白
///
/// 发布方拼字符串时经常带出多余空格，短信/标题里很显眼。
fn filter_trim(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "trim")?;
    Ok(value.map(|v| v.trim().to_owned()))
}

/// `default`：变量缺失或值为空白时换成兜底文本
///
/// 空白也算缺失：`labels` 里塞了空串是常见情况，直接代入会渲染出一个洞。
fn filter_default(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    let fallback = arg.ok_or_else(|| "default 需要兜底文本，如 {key:default(未知)}".to_owned())?;
    Ok(match value {
        Some(v) if !v.trim().is_empty() => Some(v.to_owned()),
        _ => Some(fallback.to_owned()),
    })
}

/// `truncate`：按字符截断，超长时补 `…`
///
/// 按字符而不是字节：中文按字节切会切出半个字，直接 panic 或乱码。
fn filter_truncate(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    let limit = parse_arg_usize(arg, "truncate 需要字符数，如 {key:truncate(10)}")?;
    Ok(value.map(|v| {
        if v.chars().count() <= limit {
            v.to_owned()
        } else {
            v.chars().take(limit).chain(std::iter::once('…')).collect()
        }
    }))
}

/// `fixed`：数字按固定小数位输出
///
/// 坐标这类值发布方可能给 `24.465912345`，投到短信里没必要这么长。
fn filter_fixed(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    let digits = parse_arg_usize(arg, "fixed 需要小数位，如 {key:fixed(2)}")?;
    Ok(value.map(|v| match v.trim().parse::<f64>() {
        Ok(num) => format!("{num:.digits$}"),
        Err(_) => {
            warn!("模板值 {v} 不是数字，fixed 跳过");
            v.to_owned()
        }
    }))
}

/// `mask`：脱敏，保留首尾若干字符，中间换成 `*`
///
/// 参数形如 `3,4`（保留前 3 位、后 4 位），手机号 `13800138000` 渲染成
/// `138****8000`。字符数不超过首尾之和时整串打码。
fn filter_mask(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    let arg = arg
        .filter(|a| !a.trim().is_empty())
        .ok_or_else(|| "mask 需要保留位数，如 {key:mask(3,4)}".to_owned())?;
    let (head, tail) = arg
        .split_once(',')
        .ok_or_else(|| format!("mask 的参数 {arg} 应形如 3,4"))?;
    let head = parse_arg_usize(Some(head), "mask 缺少前段保留位数")?;
    let tail = parse_arg_usize(Some(tail), "mask 缺少后段保留位数")?;

    Ok(value.map(|v| {
        let chars: Vec<char> = v.chars().collect();
        if chars.len() <= head + tail {
            return "*".repeat(chars.len());
        }
        let mut masked: String = chars[..head].iter().collect();
        masked.push_str(&"*".repeat(chars.len() - head - tail));
        masked.push_str(&chars[chars.len() - tail..].iter().collect::<String>());
        masked
    }))
}

/// # 按 RFC 3986 的 unreserved 规则做 percent-encode
///
/// 空格编成 `%20` 而不是 `+`：`+` 只在 `application/x-www-form-urlencoded`
/// 里有「空格」的含义，拼查询串之外的场景会变成字面量加号。
///
/// 手写而不引第三方 crate：规则就这一段，多一个依赖不值。
fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut encoded = String::with_capacity(value.len() * 3);
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push('%');
                encoded.push(HEX[(byte >> 4) as usize] as char);
                encoded.push(HEX[(byte & 0x0F) as usize] as char);
            }
        }
    }
    encoded
}

/// `url`：percent-encode，用于把变量拼进链接的查询参数
fn filter_url(
    value: Option<&str>,
    arg: Option<&str>,
    _ctx: &FilterCtx,
) -> Result<Option<String>, String> {
    no_arg(arg, "url")?;
    Ok(value.map(percent_encode))
}

/// # 渲染模板字符串
///
/// ## 变量来源与优先级
///
/// 内置变量（`_` 开头，服务端注入）> `labels` > `annotations`。
///
/// `labels` 与 `annotations` 都是发布方给的 key-value，模板只认键名，不关心
/// 它来自哪一份。同名时以 `labels` 为准——`labels` 是发布方声明的结构化事实，
/// `annotations` 里的同名键通常是它的可读副本。
///
/// 渲染完就只剩标题、正文与附加内容，`labels` / `annotations` 不再随投递
/// 下发：它们是模板的输入，投递的产物是渲染结果。
///
/// ## 三种失败的处理
///
/// - **变量缺失**：链里没有 `default` 时保留原占位符并 warn（人看的正文里能
///   一眼看出来）；`json` 会把它变成 `null`（保证 `ext` 仍是合法 JSON）。
/// - **过滤器名写错或语法不对**：整条占位符保留原文并 warn。这是配置错误，
///   渲染出一个看似正常的空值反而更难发现。
/// - **单个过滤器做不了**（如 `time` 遇到非数字）：warn 后跳过它，把当前值
///   交给后面的过滤器，消息照常投递。
///
/// ## 用法
///
/// ```text
/// 你好 {name}，设备 {deviceName:json} 于 {_create_ms:time(%m-%d %H:%M)} 报警
/// {"text":{deviceName:json},"time":{triggerMs:time(%F %T)|json}}
/// 链接 https://x/alarm?code={locatorCode:url}
/// 最后上报 {lastSeenMs:since}
/// ```
pub(crate) fn render_template(
    template: &str,
    builtins: &HashMap<&'static str, String>,
    labels: &HashMap<String, String>,
    annotations: &HashMap<String, String>,
) -> String {
    // 每条消息都读一次：配置热更新后下一次渲染即生效，不用重启
    let ctx = FilterCtx {
        time_offset: get_msg_config().effective_time_offset(),
    };

    TEMPLATE_VAR_RE
        .replace_all(template, |caps: &regex::Captures| {
            let key = &caps[1];
            let spec = caps.get(2).map_or("", |m| m.as_str());
            let calls = match parse_filters(spec) {
                Ok(calls) => calls,
                Err(reason) => {
                    warn!("模板变量 {{{key}:{spec}}} 语法错误（{reason}），保留占位符");
                    return caps[0].to_owned();
                }
            };

            if let Some(call) = calls.iter().find(|call| !FILTERS.contains_key(call.name)) {
                warn!("模板变量 {{{key}:{}}} 含未知过滤器，保留占位符", call.name);
                return caps[0].to_owned();
            }

            let mut value = builtins
                .get(key)
                .cloned()
                .or_else(|| labels.get(key).cloned())
                .or_else(|| annotations.get(key).cloned());

            for call in &calls {
                match FILTERS[call.name](value.as_deref(), call.arg, &ctx) {
                    Ok(next) => value = next,
                    Err(reason) => {
                        warn!("模板变量 {{{key}:{}}} 处理失败：{reason}，跳过", call.name)
                    }
                }
            }

            match value {
                Some(v) => v,
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

    /// 渲染一个模板，无内置变量、无 annotations
    fn render(template: &str, pairs: &[(&str, &str)]) -> String {
        render_template(template, &HashMap::new(), &vars(pairs), &HashMap::new())
    }

    /// 渲染一个模板，带上服务端注入的内置变量
    fn render_with_builtins(
        template: &str,
        pairs: &[(&str, &str)],
        builtins: &[(&'static str, &str)],
    ) -> String {
        let builtins: HashMap<&'static str, String> = builtins
            .iter()
            .map(|(k, v)| (*k, (*v).to_string()))
            .collect();
        render_template(template, &builtins, &vars(pairs), &HashMap::new())
    }

    #[test]
    fn test_render_template_with_all_variables() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let labels = vars(&[("name", "张三"), ("order_id", "ORD-001")]);
        let result = render_template(template, &HashMap::new(), &labels, &HashMap::new());
        assert_eq!(result, "你好 张三，你的订单 ORD-001 已发货");
    }

    #[test]
    fn test_render_template_with_missing_variable() {
        let template = "你好 {name}，你的订单 {order_id} 已发货";
        let result = render_template(template, &HashMap::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(result, "你好 {name}，你的订单 {order_id} 已发货");
    }

    #[test]
    fn test_render_template_no_placeholders() {
        let template = "系统通知：服务已重启";
        let result = render_template(template, &HashMap::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(result, "系统通知：服务已重启");
    }

    #[test]
    fn test_render_template_variables_from_annotations() {
        // 定位消息的坐标只在 annotations 里，模板照样能取到
        let template = "定位卡 {locatorCode} 位于 {lat},{lon}";
        let labels = vars(&[("locatorCode", "L001")]);
        let annotations = vars(&[("lat", "24.4659"), ("lon", "109.7192")]);
        let result = render_template(template, &HashMap::new(), &labels, &annotations);
        assert_eq!(result, "定位卡 L001 位于 24.4659,109.7192");
    }

    #[test]
    fn test_render_template_labels_win_on_conflict() {
        let labels = vars(&[("locatorCode", "L001")]);
        let annotations = vars(&[("locatorCode", "L999")]);
        let result = render_template("{locatorCode}", &HashMap::new(), &labels, &annotations);
        assert_eq!(result, "L001");
    }

    /// 内置变量是服务端事实，发布方写同名键也盖不掉
    #[test]
    fn test_render_template_builtin_wins_over_labels() {
        let labels = vars(&[("_create_ms", "0")]);
        let builtins: HashMap<&'static str, String> =
            HashMap::from([("_create_ms", "1735689600000".to_string())]);
        let result = render_template("{_create_ms}", &builtins, &labels, &HashMap::new());
        assert_eq!(result, "1735689600000");
    }

    /// 内置变量可以直接接过滤器，发布方不用自己塞时间戳
    #[test]
    fn test_render_template_builtin_with_filter() {
        let result = render_with_builtins(
            "创建于 {_create_ms:time(%Y-%m-%d %H:%M)}",
            &[],
            &[("_create_ms", "1735689600000")],
        );
        assert_eq!(result, "创建于 2025-01-01 08:00");
    }

    /// 值里带引号时，`:json` 必须转义，否则整段 JSON 被撑破
    #[test]
    fn test_render_template_json_escapes_quotes() {
        let labels = vars(&[("deviceName", "1F \"东\" 区烟感")]);
        let template = r#"{"text":{deviceName:json}}"#;
        let result = render_template(template, &HashMap::new(), &labels, &HashMap::new());
        assert_eq!(result, r#"{"text":"1F \"东\" 区烟感"}"#);
        // 关键断言：渲染结果本身是合法 JSON
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert_eq!(parsed["text"], "1F \"东\" 区烟感");
    }

    /// 数字不加引号：`:json` 只给字符串用，数值占位符保持裸代入
    #[test]
    fn test_render_template_json_with_numeric_placeholder() {
        let annotations = vars(&[("lon", "113.9"), ("lat", "22.5")]);
        let template = r#"{"lon":{lon},"lat":{lat}}"#;
        let result = render_template(template, &HashMap::new(), &HashMap::new(), &annotations);
        assert_eq!(result, r#"{"lon":113.9,"lat":22.5}"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert_eq!(parsed["lon"], 113.9);
    }

    /// 缺变量时 `:json` 输出 null，JSON 结构还在；裸占位符保留原样
    #[test]
    fn test_render_template_json_missing_variable_is_null() {
        let labels = vars(&[("locatorCode", "L001")]);
        let template = r#"{"code":{locatorCode:json},"level":{floor:json}}"#;
        let result = render_template(template, &HashMap::new(), &labels, &HashMap::new());
        assert_eq!(result, r#"{"code":"L001","level":null}"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert!(parsed["level"].is_null());
    }

    /// 默认格式：2025-01-01T00:00:00Z 在 +08:00 下是当天 08:00
    #[test]
    fn test_render_template_time_default_format() {
        let result = render(
            "触发时间 {triggerMs:time}",
            &[("triggerMs", "1735689600000")],
        );
        assert_eq!(result, "触发时间 2025-01-01 08:00:00");
    }

    /// 参数在括号里，可以带冒号和空格
    #[test]
    fn test_render_template_time_custom_format() {
        let result = render(
            "{triggerMs:time(%Y年%m月%d日 %H:%M:%S)}",
            &[("triggerMs", "1735689600000")],
        );
        assert_eq!(result, "2025年01月01日 08:00:00");
    }

    /// 秒级时间戳自动按秒解释，不会被当成 1970 年的毫秒
    #[test]
    fn test_render_template_time_accepts_seconds() {
        let result = render("{triggerMs:time}", &[("triggerMs", "1735689600")]);
        assert_eq!(result, "2025-01-01 08:00:00");
    }

    /// 格式化和 JSON 转义可以串起来：结果要能直接嵌进 JSON 字符串
    #[test]
    fn test_render_template_time_then_json() {
        let template = r#"{"time":{triggerMs:time(%F %T)|json}}"#;
        let result = render(template, &[("triggerMs", "1735689600000")]);
        assert_eq!(result, r#"{"time":"2025-01-01 08:00:00"}"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert_eq!(parsed["time"], "2025-01-01 08:00:00");
    }

    /// 值不是时间戳时跳过 time，后续的 json 仍然生效（结构不塌）
    #[test]
    fn test_render_template_time_on_non_numeric_value() {
        let template = r#"{"time":{triggerMs:time|json}}"#;
        let result = render(template, &[("triggerMs", "刚刚")]);
        assert_eq!(result, r#"{"time":"刚刚"}"#);
    }

    /// 拼错的过滤器保留原占位符，便于运营在正文里一眼发现
    #[test]
    fn test_render_template_unknown_filter_keeps_placeholder() {
        let result = render("{triggerMs:times}", &[("triggerMs", "1735689600000")]);
        assert_eq!(result, "{triggerMs:times}");
    }

    /// 括号不配对是语法错误：保留占位符，别渲染出半截结果
    #[test]
    fn test_render_template_unclosed_paren_keeps_placeholder() {
        let result = render("{t:time(%F %T}", &[("t", "1735689600000")]);
        assert_eq!(result, "{t:time(%F %T}");
    }

    /// 旧语法（`:` 传参）不再是合法写法，明确报错而不是静默错渲染
    #[test]
    fn test_render_template_old_arg_syntax_keeps_placeholder() {
        let result = render("{t:time:%F}", &[("t", "1735689600000")]);
        assert_eq!(result, "{t:time:%F}");
    }

    /// 缺变量时含 json 的链输出 null，保证 ext 模板仍是合法 JSON
    #[test]
    fn test_render_template_time_json_missing_variable_is_null() {
        let template = r#"{"time":{triggerMs:time(%F %T)|json}}"#;
        let result = render_template(template, &HashMap::new(), &HashMap::new(), &HashMap::new());
        assert_eq!(result, r#"{"time":null}"#);
        let parsed: serde_json::Value =
            serde_json::from_str(&result).expect("渲染结果不是合法 JSON");
        assert!(parsed["time"].is_null());
    }

    #[test]
    fn test_render_template_upper_lower_trim() {
        assert_eq!(render("{code:upper}", &[("code", "ab-1")]), "AB-1");
        assert_eq!(render("{code:lower}", &[("code", "AB-1")]), "ab-1");
        assert_eq!(render("[{name:trim}]", &[("name", "  张三 ")]), "[张三]");
        // 中文也能正确转大小写（无变化）
        assert_eq!(render("{name:upper}", &[("name", "张三")]), "张三");
    }

    /// 链式执行：先去空白再转大写
    #[test]
    fn test_render_template_filter_chain() {
        assert_eq!(
            render("{code:trim|upper}", &[("code", "  alarm-01  ")]),
            "ALARM-01"
        );
        // 分隔符两侧留空也认
        assert_eq!(render("{code:trim | upper}", &[("code", " a ")]), "A");
    }

    /// default 兜住缺失与空白两种「没值」
    #[test]
    fn test_render_template_default_filter() {
        assert_eq!(render("{floor:default(未知)}", &[]), "未知");
        assert_eq!(render("{floor:default(未知)}", &[("floor", "   ")]), "未知");
        assert_eq!(render("{floor:default(未知)}", &[("floor", "3F")]), "3F");
    }

    /// 括号语法的核心收益：参数里可以放 `|`
    #[test]
    fn test_render_template_arg_may_contain_pipe() {
        assert_eq!(render("{v:default(a|b)}", &[]), "a|b");
    }

    /// 截断按字符算，中文不会被切半个字
    #[test]
    fn test_render_template_truncate_by_chars() {
        assert_eq!(
            render("{desc:truncate(4)}", &[("desc", "一二三四五六")]),
            "一二三四…"
        );
        // 不超长时原样
        assert_eq!(
            render("{desc:truncate(10)}", &[("desc", "一二三")]),
            "一二三"
        );
    }

    /// 小数位收敛，非数字原样
    #[test]
    fn test_render_template_fixed_decimals() {
        assert_eq!(
            render("{lat:fixed(2)}", &[("lat", "24.465912345")]),
            "24.47"
        );
        assert_eq!(render("{lat:fixed(0)}", &[("lat", "24.6")]), "25");
        assert_eq!(render("{lat:fixed(2)}", &[("lat", "未知")]), "未知");
    }

    /// 脱敏：保留首尾，中间打码；过短时整串打码
    #[test]
    fn test_render_template_mask() {
        assert_eq!(
            render("{phone:mask(3,4)}", &[("phone", "13800138000")]),
            "138****8000"
        );
        assert_eq!(render("{name:mask(1,0)}", &[("name", "张三丰")]), "张**");
        // 长度不足首尾之和：全打码，不 panic
        assert_eq!(render("{phone:mask(3,4)}", &[("phone", "138")]), "***");
    }

    /// URL 编码：中文按 UTF-8 逐字节转义，空格是 %20 不是 +
    #[test]
    fn test_render_template_url_encode() {
        assert_eq!(
            render("{q:url}", &[("q", "1F 东区&x=1")]),
            "1F%20%E4%B8%9C%E5%8C%BA%26x%3D1"
        );
        // unreserved 字符原样保留
        assert_eq!(render("{q:url}", &[("q", "a-b_c.d~e")]), "a-b_c.d~e");
    }

    /// 相对时间：按最大单位描述，未来时间按「刚刚」
    #[test]
    fn test_render_template_since() {
        let now = Utc::now().timestamp_millis();
        let cases = [
            (now - 1_000, "刚刚"),
            (now - 5 * 60_000, "5 分钟前"),
            (now - 3 * 3_600_000, "3 小时前"),
            (now - 2 * 86_400_000, "2 天前"),
            (now - 45 * 86_400_000, "1 个月前"),
            (now + 60_000, "刚刚"),
        ];
        for (ts, expected) in cases {
            assert_eq!(
                render("{t:since}", &[("t", &ts.to_string())]),
                expected,
                "时间戳 {ts}"
            );
        }
    }

    /// 参数写错只跳过这一个过滤器，原值继续往右走
    #[test]
    fn test_render_template_bad_arg_is_skipped() {
        assert_eq!(
            render("{desc:truncate(abc)}", &[("desc", "一二三")]),
            "一二三"
        );
        // fixed 参数错 → 跳过 → json 仍把原值转义成合法 JSON
        assert_eq!(
            render(r#"{"v":{v:fixed(x)|json}}"#, &[("v", "1.5")]),
            r#"{"v":"1.5"}"#
        );
    }

    /// 不带参数的过滤器被塞了参数：跳过，不改变值
    #[test]
    fn test_render_template_unexpected_arg_is_skipped() {
        assert_eq!(render("{code:upper(1)}", &[("code", "ab")]), "ab");
    }

    /// 空过滤器（`{key:}`）等同于原样代入，不报错
    #[test]
    fn test_render_template_empty_filter_spec() {
        assert_eq!(render("{code:}", &[("code", "ab")]), "ab");
    }
}
