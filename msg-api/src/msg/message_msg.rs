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
    /// 路由标签：一组结构化的 key-value，匹配、分组、路由都只看它
    ///
    /// 同时是模板变量：`title_template` / `content_template` 里的 `{key}`
    /// 会从 `labels` 与 [`Self::annotations`] 中取值。渲染之后它仍然原样跟着
    /// 投递任务下发（见 [`crate::msg::DeliveryMsg::labels`]），接收方还要拿它
    /// 做业务联动。
    #[serde(default)]
    #[builder(default)]
    pub labels: HashMap<String, String>,
    /// 可读标注：标题、详细描述、坐标这类展示用字段
    ///
    /// 与 `labels` 一样，既是模板的输入变量（同名时以 `labels` 为准），
    /// 也是投递任务的一部分：大屏拿 `lon` / `lat` / `levelId` 在地图上落点，
    /// 这些字段拼不进标题，只能原样下发。
    #[serde(default)]
    #[builder(default)]
    pub annotations: HashMap<String, String>,
}
