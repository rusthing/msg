//! # 投递任务
//!
//! `msg-svr` 渲染完消息后，按「目标 × 渠道」拆任务，发布到
//! `msg.delivery.{渠道编码}`，由该渠道的发送方消费。
//!
//! ## 持久化与否决定整条链路的语义
//!
//! `persisted` 取自 `msg_message.persisted`：
//!
//! - `true`：消息已落 `msg_delivery` / `msg_delivery_target` / `msg_delivery_channel`，
//!   任务里带齐三级 ID，发送结果要回写状态；发送失败可重投。
//! - `false`：**不落库、不回写、不重投**。任务里的三级 ID 全为 `None`，
//!   发送方推不出去就直接丢弃，丢了就是丢了。
//!
//! 因此消费方必须先看 `persisted` 再决定要不要写库，不能默认 ID 一定存在。

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// 一次投递任务（一个目标的一个渠道）
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryMsg {
    /// 消息是否持久化
    ///
    /// `false` 时下面三个投递 ID 均为 `None`，发送方不需要、也不应该回写状态。
    pub persisted: bool,

    /// 投递记录 ID（`msg_delivery.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_id: Option<u64>,
    /// 投递目标 ID（`msg_delivery_target.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_target_id: Option<u64>,
    /// 投递渠道 ID（`msg_delivery_channel.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_channel_id: Option<u64>,

    /// 事件编码
    pub event_code: String,
    /// 消息 ID（`msg_message.id`）
    pub message_id: u64,
    /// 渠道 ID（`msg_channel.id`）
    pub channel_id: u64,
    /// 渠道编码，同时是发布主题的末段
    pub channel_code: String,
    /// 目标 ID
    ///
    /// websocket 渠道用它定位长连接：大屏连接上来时上报的 `targetId` 就是这个值。
    pub target_id: u64,

    /// 业务 ID
    ///
    /// 与来源消息的 `business_id` 同值。非持久化消息的发布方没有业务对象可指，
    /// 这里就是 `None`；持久化消息必然有值（没有的话在幂等阶段就被拒了）。
    #[serde(default)]
    pub business_id: Option<u64>,
    /// 业务触发时间戳（毫秒）
    ///
    /// 同 [`Self::business_id`]：非持久化消息可为 `None`。
    #[serde(default)]
    pub business_trigger_ms: Option<u64>,

    /// 已渲染的标题
    ///
    /// 模板把 `labels` / `annotations` 的变量注入后得到的结果，是投递的唯一
    /// 内容载体。渠道拿到就能直接发，不需要再读原始变量。
    pub title: String,
    /// 已渲染的内容
    ///
    /// 同 [`Self::title`]：渲染即编译，产物下发，输入不留。
    pub content: String,
    /// 消息生成时间戳（毫秒）
    pub created_ms: u64,
}
