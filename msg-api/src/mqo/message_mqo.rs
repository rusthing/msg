use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// 从 NATS 收到的消息负载
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct MessageMqo {
    pub event_code: String,
    pub business_id: u64,
    /// 业务触发时间戳（毫秒）
    ///
    /// **必填**。与 `event_code`、`business_id` 一起构成幂等键：
    /// `(event_code, business_id, business_trigger_ms)` 只会产生一条投递记录。
    ///
    /// 取事件实际发生的时间，而不是发布调用的时间：
    /// - 重投/重放时必须保持不变，否则幂等失效、同一条消息会被重复投递；
    /// - 同一业务对象的第二次真实触发必须给出新的时间戳，否则会被判为重复而跳过。
    ///
    /// 缺字段的消息包会在 `msg-svr` 解析阶段直接失败（`Discard`），
    /// 由发布方保证传递，服务端不再兜底。
    pub business_trigger_ms: u64,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}
