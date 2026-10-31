use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// 从 NATS 收到的消息负载
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct MessageMsg {
    pub event_code: String,
    /// 业务 ID
    ///
    /// **可选**。与 `event_code`、`business_trigger_ms` 一起构成幂等键
    /// `(event_code, business_id, business_trigger_ms)`，三者都在时才判重。
    ///
    /// 留空的情形：发布方没有业务对象可指。不落库的消息（`persisted = false`）
    /// 填不填都不参与幂等——没有投递记录可存幂等键，语义就是 at-most-once；
    /// 有业务对象时照样可以填，服务端会原样带进投递任务。
    ///
    /// 落库的消息（`msg_message.persisted = true`）必须给出这个值，
    /// 否则 `msg-svr` 在幂等阶段直接失败（`Discard`）。
    #[serde(default)]
    pub business_id: Option<u64>,
    /// 业务触发时间戳（毫秒）
    ///
    /// **可选**，与 `business_id` 同进同退，两个都给出才参与幂等。
    ///
    /// 取事件实际发生的时间，而不是发布调用的时间：
    /// - 重投/重放时必须保持不变，否则幂等失效、同一条消息会被重复投递；
    /// - 同一业务对象的第二次真实触发必须给出新的时间戳，否则会被判为重复而跳过。
    ///
    /// 服务端不兜底：补 `now` 会让每次重投都算新消息，补 0 又会吞掉后续真实触发。
    #[serde(default)]
    pub business_trigger_ms: Option<u64>,
    /// 事件**首次**触发时间戳（毫秒），与 [`Self::business_trigger_ms`] 分工不同
    ///
    /// **可选**，不参与幂等，只用来算端到端延迟：
    ///
    /// - `business_trigger_ms` 是「这一次业务动作」的时间，跟着幂等键走，每次
    ///   通知都不同（报警重复提醒就是靠它区分）；
    /// - `trigger_ms` 是「这件事什么时候开始发生」，同一条报警通知多次都是同一个
    ///   值，所以拿它和投递完成时间相减才是有意义的端到端延迟。
    ///
    /// 发布方拿不到事件起点时留空，服务端不兜底——补 `now` 会让延迟恒为 0，
    /// 补 `business_trigger_ms` 又等于把「通知时间」冒充「事件时间」。
    #[serde(default)]
    #[builder(default)]
    pub trigger_ms: Option<u64>,
    /// 路由标签：一组结构化的 key-value，匹配、分组、路由都只看它
    ///
    /// **只是模板变量**：`title_template` / `content_template` /
    /// `ext_template` 里的 `{key}` 会从 `labels` 与 [`Self::annotations`]
    /// 中取值（同名时以 `labels` 为准）。
    ///
    /// `_` 开头的键是**保留命名空间**：msg-svr 会注入 `_create_ms`、
    /// `_business_trigger_ms`、`_business_id`、`_message_id`、`_event_code`，
    /// 优先级高于这里，发布方写同名键会被忽略。时间戳不必自己塞——模板里
    /// `{_create_ms:time(%F %T)}` 直接可用。
    ///
    /// 渲染完就不再随投递任务下发，只留在 `msg_delivery.labels` 的快照里
    /// 供排查。接收方要用的字段，必须写进模板变成 `content` 或 `ext`。
    #[serde(default)]
    #[builder(default)]
    pub labels: HashMap<String, String>,
    /// 可读标注：详细描述、坐标这类展示用字段
    ///
    /// 与 `labels` 一样只是模板的输入变量（同名时以 `labels` 为准），
    /// 不随投递下发。大屏要落点，就得把坐标写进 `content_template`，
    /// 由模板渲染成 JSON 文本交给大屏解析。
    #[serde(default)]
    #[builder(default)]
    pub annotations: HashMap<String, String>,
    /// 发布方自带的标题
    ///
    /// **可选**。`msg_message.title_template` 有值时以模板渲染结果为准，
    /// 这个值只在模板为 `null` 时使用。两边都空 `msg-svr` 直接拒收。
    #[serde(default)]
    #[builder(default)]
    pub title: Option<String>,
    /// 发布方自带的内容
    ///
    /// **可选**，取舍规则同 [`Self::title`]。
    #[serde(default)]
    #[builder(default)]
    pub content: Option<String>,
    /// 发布方自带的附加内容
    ///
    /// **可选**，取舍规则同 [`Self::title`]。格式不做任何约定：
    /// 可以是 JSON、可以是人话，接收方自己判断怎么解。
    #[serde(default)]
    #[builder(default)]
    pub ext: Option<String>,
}
