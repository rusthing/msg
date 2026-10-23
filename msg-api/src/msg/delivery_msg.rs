//! # 投递任务
//!
//! `msg-svr` 渲染完消息后，按「目标 × 渠道」拆任务，发布到
//! `msg.delivery.{渠道编码}`，由该渠道的发送方消费。
//!
//! ## 任务里带什么
//!
//! 字段与投递三张表一一对应，没有额外加工出来的概念：
//!
//! - 消息内容：`msg_delivery` 的 `event_code` / `message_id` / `event_source_id` /
//!   `message_category_id` / `business_id` / `business_trigger_ms` / `labels` /
//!   `annotations` / `title` / `content` / `create_ms`；
//! - 投递目标：`msg_delivery_target` 的 `target_category_id` / `target_id`；
//! - 投递渠道：`msg_delivery_channel` 的 `channel_id`，外加 `msg_channel.code`
//!   （发布主题的末段，消费方靠它认领任务）。
//!
//! `labels` / `annotations` 是发布方原样上报的结构化数据：模板拿它当变量拼文案，
//! 接收方也拿它做业务处理（大屏按 `lon` / `lat` 落点、按 `levelId` 切楼层）。
//! 渲染出标题不等于原始信息就没用了，两者一起下发。
//!
//! ## 标题与内容可以为空
//!
//! `title` / `content` 取自 `msg_message.title_template` / `content_template`：
//! 有模板就有值，模板为 `null` 就没有，不做额外兜底。只剩结构化数据的消息
//! （纯遥测）照样投递，接收方按 `labels` / `annotations` 自己处理。
//!
//! ## 持久化与否决定整条链路的语义
//!
//! `persisted` 取自 `msg_message.persisted`：
//!
//! - `true`：消息已落 `msg_delivery` / `msg_delivery_target` / `msg_delivery_channel`，
//!   任务里带齐三张表的行 ID，发送结果要回写状态；发送失败可重投。
//! - `false`：**不落库、不回写、不重投**。任务里的三张表行 ID 全为 `None`，
//!   发送方推不出去就直接丢弃，丢了就是丢了。
//!
//! 因此消费方必须先看 `persisted` 再决定要不要写库，不能默认 ID 一定存在。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use typed_builder::TypedBuilder;

/// 一次投递任务（一个目标的一个渠道）
#[derive(Debug, Serialize, Deserialize, Clone, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryMsg {
    /// 消息是否持久化（`msg_message.persisted`）
    ///
    /// `false` 时下面三个行 ID 均为 `None`，发送方不需要、也不应该回写状态。
    pub persisted: bool,

    /// 投递记录 ID（`msg_delivery.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_id: Option<u64>,
    /// 投递目标记录 ID（`msg_delivery_target.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_target_id: Option<u64>,
    /// 投递渠道记录 ID（`msg_delivery_channel.id`），非持久化消息为 `None`
    #[serde(default)]
    pub delivery_channel_id: Option<u64>,

    /// 事件编码（`msg_delivery.event_code`）
    pub event_code: String,
    /// 消息 ID（`msg_delivery.message_id`）
    pub message_id: u64,
    /// 事件来源 ID（`msg_delivery.event_source_id`）
    pub event_source_id: u64,
    /// 消息类别 ID（`msg_delivery.message_category_id`）
    pub message_category_id: u64,
    /// 业务 ID（`msg_delivery.business_id`）
    ///
    /// 与来源消息的 `business_id` 同值，发布方没给就是 `None`。
    /// 持久化消息必然有值（缺了在幂等阶段就被拒）；非持久化消息可有可无，
    /// 有的话同样下发（定位消息就带着绑定目标 ID），只是不参与幂等。
    #[serde(default)]
    pub business_id: Option<u64>,
    /// 业务触发时间戳（毫秒，`msg_delivery.business_trigger_ms`）
    ///
    /// 同 [`Self::business_id`]：非持久化消息可为 `None`。
    #[serde(default)]
    pub business_trigger_ms: Option<u64>,

    /// 路由标签（`msg_delivery.labels` 的快照）
    ///
    /// 发布方原样上报的 key-value：既是模板变量，也是投递内容的一部分，
    /// 接收方按 key 取业务字段做联动，不依赖标题里有没有拼出来。
    #[serde(default)]
    #[builder(default)]
    pub labels: HashMap<String, String>,
    /// 可读标注（`msg_delivery.annotations` 的快照）
    ///
    /// 同 [`Self::labels`]：模板变量与投递内容两用。坐标、楼层、绑定对象
    /// 这些展示侧要用的字段都在这里，大屏拿它落点、画图标。
    #[serde(default)]
    #[builder(default)]
    pub annotations: HashMap<String, String>,

    /// 已渲染的标题（`msg_delivery.title`）
    ///
    /// 由 `msg_message.title_template` 渲染得来。模板为 `null` 时是 `None`，
    /// 不从 `annotations` 兜底。
    #[serde(default)]
    pub title: Option<String>,
    /// 已渲染的内容（`msg_delivery.content`）
    ///
    /// 同 [`Self::title`]：模板决定有没有，不做兜底。
    #[serde(default)]
    pub content: Option<String>,
    /// 消息创建时间戳（毫秒，`msg_delivery.create_ms`）
    pub create_ms: u64,

    /// 投递目标类别 ID（`msg_delivery_target.target_category_id`）
    pub target_category_id: u64,
    /// 目标 ID（`msg_delivery_target.target_id`）
    ///
    /// websocket 渠道用它定位长连接：大屏连接上来时上报的 `targetId` 就是这个值。
    pub target_id: u64,
    /// 渠道 ID（`msg_delivery_channel.channel_id`）
    pub channel_id: u64,
    /// 渠道编码（`msg_channel.code`）
    ///
    /// 同时是投递主题 `msg.delivery.{code}` 的末段。
    pub channel_code: String,
}
