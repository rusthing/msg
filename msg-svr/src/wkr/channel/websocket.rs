//! # websocket 渠道发送器
//!
//! ## 帧长什么样由渠道配置决定
//!
//! 配了 `msg_channel.payload_template` 就按模板渲染，没配（或模板渲染不出可用帧）
//! 就用下面这个默认结构：
//!
//! ```json
//! {"eventCode":"…","title":"…","content":"…","ext":"…","firstReceiveMs":1761…,"businessTriggerMs":1761…}
//! ```
//!
//! 模板能用的变量只有服务端给的这几个，都是已渲染好的结果或服务端事实：
//!
//! | 变量 | 含义 |
//! |---|---|
//! | `eventCode` | 事件编码（`msg_delivery.event_code`），大屏按它决定怎么渲染 |
//! | `title` | 渲染好的标题 |
//! | `content` | 渲染好的内容；定位消息的落点就在这段 JSON 文本里 |
//! | `ext` | 渲染好的附加内容（`msg_delivery.ext`），这条消息没有就不注入 |
//! | `firstReceiveMs` | 链路第一跳收到事件的服务端时刻，发布方没带就不注入 |
//! | `businessTriggerMs` | 本次通知的业务时间（发布方时钟），没带就不注入 |
//!
//! 语法与消息模板完全一致（`{变量:过滤器}`，字符串要写成 `{title:json}` 才有引号与
//! 转义）。缺失的变量用 `{ext:json}` 渲染成 `null`；裸写 `{ext}` 会留下占位符，
//! 那就不再是合法 JSON，按下面的回退规则处理。
//!
//! 两层模板分工不同：`msg_message` 的三份模板决定「说什么」（一条消息一份），
//! `msg_channel.payload_template` 决定「怎么说话」（一个渠道一份）。换屏、换协议
//! 只改渠道配置，文案不用重配，也不用发版本。
//!
//! ## 渲染不出来就回退，不推坏帧
//!
//! 模板写错（引号没转义、变量名拼错留下占位符、渲染成空白）时告警并改用默认帧：
//! 大屏解析失败会丢掉**整帧**，比收到一份旧结构更糟。回退是静默降级，但日志里
//! 每次都会留下渠道与事件编码，配错能查到。
//!
//! ## 落点在哪
//!
//! 落点要用的坐标不在独立字段里：它们由内容模板渲染进 `content`，大屏按约定的
//! 格式解析 `content` 即可；也可以在载荷模板里用 `{content}` 原样嵌进自己的结构。
//! 发布方上报的 `labels` / `annotations` 只是模板的输入，不随这一帧下发。
//!
//! ## 两个时间锚点
//!
//! `firstReceiveMs` 是链路第一跳收到原始事件的服务端时刻，`businessTriggerMs`
//! 是本次通知的业务时间（发布方时钟）。大屏是只收不发的长连接，没有回执通道，
//! 帧里不带时间戳，它就只能拿本地收帧时刻当起点，既算不出真实延迟，也判不了
//! 「这条报警是不是早就该到了」。两个值都可能缺（发布方没接入），缺了就是
//! `null`，大屏按「不可算」处理。
//!
//! ## `ext` 是给机器看的那一份
//!
//! `title` / `content` 是人看的文案，`ext` 是同一件事的结构化版本：报警 ID、
//! 定位坐标、Unity 场景坐标这类字段。它由发布方（或消息中心的 `ext_template`）
//! 渲染成一段文本，格式不做约定——报警中心给的是 JSON，大屏自己解析。
//!
//! 默认帧里没有 `ext` 的消息整个字段都不出现（`skip_serializing_if`），大屏侧按
//! 「有没有 `ext`」判断这条消息带不带结构化数据，不需要区分 `null` 和空串。
//! 换成模板帧后这个字段由模板决定：`{ext:json}` 缺失时渲染 `null`。
//!
//! ## 不下发什么
//!
//! 投递三张表的行 ID 与目标/渠道字段不进默认帧：大屏是只收不发的长连接
//! （见 [`crate::wkr::ws_handler`]），拿不到回执，这些字段没有任何消费方。
//! 模板同样拿不到它们——变量只有上面那几个。
//!
//! 目标类别下没有连接（或压根没连）时返回 [`SendOutcome::NoConnection`]：
//! websocket 是 at-most-once，没有连接就没有必要重试，也不需要补投——
//! 大屏自己刷新列表即可。

use super::{ChannelSender, SendOutcome};
use crate::cache::get_message_cache;
use crate::utils::render_template;
use crate::wkr::conn;
use async_trait::async_trait;
use msg_api::cst::MSG_CHANNEL_CODE_WEBSOCKET;
use msg_api::msg::DeliveryMsg;
use serde::Serialize;
use std::collections::HashMap;
use tracing::warn;

/// websocket 渠道发送器
pub struct WebSocketSender;

#[async_trait]
impl ChannelSender for WebSocketSender {
    fn code(&self) -> &'static str {
        MSG_CHANNEL_CODE_WEBSOCKET
    }

    async fn send(&self, task: &DeliveryMsg) -> SendOutcome {
        let text = match frame(task) {
            Ok(text) => text,
            Err(e) => return SendOutcome::Failed(format!("推送内容序列化失败: {e}")),
        };

        // 按目标类别编码找连接：大屏连上来时上报的就是这个编码，
        // 本实例把这个类别下的连接全发一遍，别的实例收到同一条任务也会各自发
        if conn::send(&task.target_category_code, text) {
            // 地址只用于排查：这条投递落在哪个类别的连接上
            SendOutcome::Delivered {
                address: format!("category:{}", task.target_category_code),
            }
        } else {
            // 「本实例没有连接」不是投递失败：屏可能连在别的实例上，
            // 那边会把这条渠道记成送达
            warn!(
                "本实例没有该类别下的连接，消息丢弃: category={}, event_code={}",
                task.target_category_code, task.event_code
            );
            SendOutcome::NoConnection
        }
    }
}

/// # 组一帧
///
/// 渠道配了载荷模板就按模板渲染，模板缺席或渲染不出可用帧时回退默认结构。
///
/// 模板从消息缓存里取（按事件编码找到消息、再按渠道编码找到渠道），不读库：
/// 位置更新这类消息一秒几十上百条，每条都查一次库不值。渠道写操作会触发缓存
/// 全量重建，改模板不用重启。
fn frame(task: &DeliveryMsg) -> Result<String, String> {
    if let Some(template) = payload_template(&task.event_code, &task.channel_code)
        && let Some(text) = render_payload(&template, task)
    {
        return Ok(text);
    }

    default_frame(task)
}

/// # 取该渠道的载荷模板
///
/// ## 返回值
///
/// - `Some(模板)`：渠道配了 `payload_template`；
/// - `None`：没配（用默认帧），或缓存里找不到这条消息/渠道——后者只在缓存刚被
///   重建、渠道已解绑时才发生，同样按「没配」处理，不因为一次配置变更丢消息。
fn payload_template(event_code: &str, channel_code: &str) -> Option<String> {
    get_message_cache()
        .get(event_code)?
        .channels
        .iter()
        .find(|channel| channel.code == channel_code)?
        .payload_template
        .clone()
}

/// # 按模板渲染一帧
///
/// ## 参数
///
/// - `template`：渠道的载荷模板；
/// - `task`：投递任务，提供 `eventCode` / `title` / `content` / `ext` /
///   `firstReceiveMs` / `businessTriggerMs` 六个变量。
///
/// ## 返回值
///
/// - `Some(文本)`：渲染结果非空且是合法 JSON；
/// - `None`：渲染成空白，或不是合法 JSON——调用方回退默认帧。
///
/// ## 为什么要求合法 JSON
///
/// websocket 这条渠道的约定就是 JSON：大屏 `JSON.parse` 整帧，解析失败会丢掉
/// 整帧（包括里面的定位数据），比收到一份旧结构的帧更糟。校验用
/// [`serde::de::IgnoredAny`] 只走一遍语法，不建 DOM——热路径上省一次内存分配。
fn render_payload(template: &str, task: &DeliveryMsg) -> Option<String> {
    let mut builtins: HashMap<&'static str, String> = HashMap::from([
        ("eventCode", task.event_code.clone()),
        ("title", task.title.clone()),
        ("content", task.content.clone()),
    ]);

    // 缺的变量不注入：模板里 `{ext:json}` 会渲染成 null，与「这条消息没有附加
    // 内容 / 算不出时间」一致。注入空串反而会渲染出 `"ext":""`，让大屏把空串
    // 当成一份有效的附加内容。
    if let Some(ext) = task.ext.as_deref() {
        builtins.insert("ext", ext.to_owned());
    }
    if let Some(first_receive_ms) = task.first_receive_ms {
        builtins.insert("firstReceiveMs", first_receive_ms.to_string());
    }
    if let Some(business_trigger_ms) = task.business_trigger_ms {
        builtins.insert("businessTriggerMs", business_trigger_ms.to_string());
    }

    // 载荷模板只吃服务端给的变量，不接发布方的 labels / annotations：
    // 那两份是消息模板的输入，到这一层已经渲染完了。
    let rendered = render_template(template, &builtins, &HashMap::new(), &HashMap::new());

    if rendered.trim().is_empty() {
        warn!(
            "渠道载荷模板渲染为空，改用默认帧: channel={}, event_code={}",
            task.channel_code, task.event_code
        );
        return None;
    }

    if serde_json::from_str::<serde::de::IgnoredAny>(&rendered).is_err() {
        warn!(
            "渠道载荷模板渲染结果不是合法 JSON，改用默认帧: channel={}, event_code={}, rendered={rendered}",
            task.channel_code, task.event_code
        );
        return None;
    }

    Some(rendered)
}

/// # 默认帧
///
/// 渠道没配载荷模板时的结构，也是模板写错时的兜底。只放渲染结果 + 两个时间
/// 锚点：字段越少，大屏侧解析越稳，帧也越小。
///
/// 时间锚点是大屏唯一能自己算延迟的地方：它拿不到库，也没有回执通道，
/// 想判断「这条报警从事件发生到现在过了多久」「该不该判过期不弹」，
/// 只能靠帧里带下来的时间戳。
fn default_frame(task: &DeliveryMsg) -> Result<String, String> {
    let push = WsPush {
        event_code: &task.event_code,
        title: &task.title,
        content: &task.content,
        ext: task.ext.as_deref(),
        first_receive_ms: task.first_receive_ms,
        business_trigger_ms: task.business_trigger_ms,
    };

    serde_json::to_string(&push).map_err(|e| e.to_string())
}

/// 推给大屏的一帧（默认结构）
///
/// 配了 `msg_channel.payload_template` 的渠道不用它，结构由模板决定。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct WsPush<'a> {
    /// 事件编码（`msg_delivery.event_code`），大屏按它决定怎么渲染
    event_code: &'a str,
    /// 渲染好的标题
    title: &'a str,
    /// 渲染好的内容；定位消息的落点就在这段 JSON 文本里
    content: &'a str,
    /// 渲染好的附加内容（`msg_delivery.ext`），没有就不出现在帧里
    #[serde(skip_serializing_if = "Option::is_none")]
    ext: Option<&'a str>,
    /// 链路第一跳收到原始事件的时刻（毫秒，服务端时钟）
    ///
    /// 端到端延迟只用它算：它是链路上第一个可信的服务端时间，不受终端时钟
    /// 偏差、离线补传影响。发布方没带就是 `null`，大屏按「不可算」处理。
    #[serde(skip_serializing_if = "Option::is_none")]
    first_receive_ms: Option<u64>,
    /// 本次通知的业务时间（毫秒，发布方时钟）
    ///
    /// 与业务库的幂等键同值，大屏回查报警详情时用它对齐同一次通知；
    /// 不适合单独用来算延迟（时钟可能偏）。
    #[serde(skip_serializing_if = "Option::is_none")]
    business_trigger_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> DeliveryMsg {
        DeliveryMsg {
            persisted: false,
            delivery_id: None,
            delivery_target_id: None,
            delivery_channel_id: None,
            event_code: "position.updated".to_string(),
            message_id: 7,
            event_source_id: 3,
            message_category_id: 5,
            business_id: None,
            business_trigger_ms: None,
            first_trigger_ms: None,
            first_receive_ms: None,
            title: "张三 进入 1F".to_string(),
            // 定位消息的落点就在内容里，大屏靠它在地图上定位
            content: r#"{"text":"1F 东侧","lon":113.9,"lat":22.5}"#.to_string(),
            ext: None,
            create_ms: 1_700_000_000_000,
            target_category_code: "screen".to_string(),
            target_category_id: 11,
            // 空串 = 该类别下的全部目标（按类别广播）
            target_id: String::new(),
            channel_id: 2,
            channel_code: "websocket".to_string(),
        }
    }

    /// 默认帧解析成 JSON
    fn default_json(task: &DeliveryMsg) -> serde_json::Value {
        let text = default_frame(task).expect("帧序列化失败");
        serde_json::from_str(&text).expect("帧反序列化失败")
    }

    /// 模板渲染结果解析成 JSON
    fn parse(text: &str) -> serde_json::Value {
        serde_json::from_str(text).expect("渲染结果不是合法 JSON")
    }

    /// 没配载荷模板（缓存里查不到这个渠道）时还是原来那一帧
    #[test]
    fn absent_template_uses_default_frame() {
        assert_eq!(
            frame(&task()).expect("帧序列化失败"),
            default_frame(&task()).expect("帧序列化失败")
        );
    }

    /// 帧里必须带时间锚点：大屏没有回执通道，不从帧里拿时间就只能拿本机收帧
    /// 时刻当起点，端到端延迟永远算成 0
    #[test]
    fn frame_carries_time_anchors() {
        let task = DeliveryMsg {
            first_receive_ms: Some(1_761_000_000_000),
            business_trigger_ms: Some(1_760_999_990_000),
            ..task()
        };

        let frame = default_json(&task);

        assert_eq!(frame["firstReceiveMs"], 1_761_000_000_000i64);
        assert_eq!(frame["businessTriggerMs"], 1_760_999_990_000i64);
    }

    /// 两个锚点都没有时字段整段消失：大屏按「有没有」判断能不能算延迟
    #[test]
    fn absent_time_anchors_are_not_serialized() {
        let frame = default_json(&task());

        assert!(frame.get("firstReceiveMs").is_none());
        assert!(frame.get("businessTriggerMs").is_none());
    }

    /// 一帧放三个渲染结果：事件编码、标题、内容
    #[test]
    fn frame_carries_render_result() {
        let frame = default_json(&task());

        assert_eq!(frame["eventCode"], "position.updated");
        assert_eq!(frame["title"], "张三 进入 1F");
        // 内容是 JSON 文本，原样透传，不在推送侧二次解析
        let content: serde_json::Value =
            serde_json::from_str(frame["content"].as_str().expect("content 不是字符串"))
                .expect("content 不是合法 JSON");
        assert_eq!(content["lon"], 113.9);
    }

    /// `ext` 有就原样带下去：大屏靠它拿 alertId 与 Unity 坐标
    #[test]
    fn frame_carries_ext() {
        let task = DeliveryMsg {
            ext: Some(r#"{"alertId":"7","unityX":"1.5"}"#.to_string()),
            ..task()
        };

        let frame = default_json(&task);
        let ext: serde_json::Value =
            serde_json::from_str(frame["ext"].as_str().expect("ext 不是字符串"))
                .expect("ext 不是合法 JSON");
        assert_eq!(ext["alertId"], "7");
    }

    /// 没有 `ext` 时整个字段不出现：大屏按「有没有 ext」判断，不该收到 null
    #[test]
    fn absent_ext_is_not_serialized() {
        let frame = default_json(&task());
        assert!(frame.get("ext").is_none());
    }

    /// 投递元数据不下发：任务里有行 ID、目标/渠道字段，帧里一个都不能出现
    #[test]
    fn delivery_metadata_is_not_pushed() {
        let mut keys: Vec<String> = default_json(&task())
            .as_object()
            .expect("帧不是对象")
            .keys()
            .cloned()
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, ["content", "eventCode", "title"]);
    }

    /// 模板接管帧结构：字段名、字段集、嵌套都由模板决定，不再受默认帧约束
    #[test]
    fn template_takes_over_frame_shape() {
        let text = render_payload(
            r#"{"type":{eventCode:json},"data":{"title":{title:json},"body":{content:json}}}"#,
            &task(),
        )
        .expect("模板应渲染出合法 JSON");

        let frame = parse(&text);
        assert_eq!(frame["type"], "position.updated");
        assert_eq!(frame["data"]["title"], "张三 进入 1F");
        // `:json` 把内容当字符串嵌进来（里面自带引号也被转义了）
        assert_eq!(frame["data"]["body"], task().content);
        // 默认帧那几个字段自然不存在了：模板说了算
        assert!(frame.get("eventCode").is_none());
    }

    /// 内容本身是 JSON 文本时可以原样嵌进自己的结构，大屏少解一层
    #[test]
    fn template_may_inline_rendered_content() {
        let text = render_payload(r#"{"code":{eventCode:json},"data":{content}}"#, &task())
            .expect("模板应渲染出合法 JSON");

        let frame = parse(&text);
        assert_eq!(frame["data"]["lon"], 113.9);
    }

    /// 时间锚点在模板里同样是变量：没有就不用 `:json` 渲染成 null
    #[test]
    fn template_carries_time_anchors() {
        let with_anchors = DeliveryMsg {
            first_receive_ms: Some(1_761_000_000_000),
            business_trigger_ms: Some(1_760_999_990_000),
            ..task()
        };
        let template = r#"{"recv":{firstReceiveMs},"notify":{businessTriggerMs}}"#;

        let frame = parse(&render_payload(template, &with_anchors).expect("应渲染出合法 JSON"));
        assert_eq!(frame["recv"], 1_761_000_000_000i64);
        assert_eq!(frame["notify"], 1_760_999_990_000i64);

        // 缺了就得靠 `:json` 才渲染成 null；裸占位符会留下 `{firstReceiveMs}` 字面量，
        // 整帧判为非法 JSON → 回退默认帧（不会带着占位符推给大屏）
        let nullable = r#"{"recv":{firstReceiveMs:json},"notify":{businessTriggerMs:json}}"#;
        let frame = parse(&render_payload(nullable, &task()).expect("应渲染出合法 JSON"));
        assert!(frame["recv"].is_null());
        assert!(render_payload(template, &task()).is_none());
    }

    /// ext 缺失时渲染成 null，帧仍然合法
    #[test]
    fn template_missing_ext_renders_null() {
        let text = render_payload(r#"{"ext":{ext:json}}"#, &task()).expect("应渲染出合法 JSON");
        assert!(parse(&text)["ext"].is_null());
    }

    /// 模板写错就不给帧：裸占位符会留下 `{key}` 字面量，解析不了的帧不能推给大屏
    #[test]
    fn template_with_unknown_variable_is_rejected() {
        assert!(render_payload(r#"{"v":{missing}}"#, &task()).is_none());
    }

    /// 字符串没用 `:json` 时引号会把 JSON 撑破，同样按「渲染不出可用帧」处理
    #[test]
    fn template_with_unescaped_string_is_rejected() {
        let task = DeliveryMsg {
            title: "1F \"东\" 区".to_string(),
            ..task()
        };
        assert!(render_payload(r#"{"title":{title}}"#, &task).is_none());
    }

    /// 渲染成空白不算一帧：空帧推过去，大屏只会当成一条空消息
    #[test]
    fn blank_template_is_rejected() {
        assert!(render_payload("   ", &task()).is_none());
    }
}
