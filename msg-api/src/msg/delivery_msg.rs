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
//!   `message_category_id` / `business_id` / `business_trigger_ms` / `title` /
//!   `content` / `ext` / `create_ms`；
//! - 投递目标：`msg_delivery_target` 的 `target_category_id` / `target_id`；
//! - 投递渠道：`msg_delivery_channel` 的 `channel_id`，外加 `msg_channel.code`
//!   （发布主题的末段，消费方靠它认领任务）。
//!
//! ## 标题与内容必填，ext 可选
//!
//! `title` / `content` 一定有值：先按 `msg_message` 的模板渲染，模板为 `null`
//! 时取发布方给的值，两边都空 `msg-svr` 直接拒收（`Discard`），不会把一条
//! 没有正文的消息发出来。
//!
//! `ext` 可以没有。模板渲染出的文本格式不做任何约定——可能是 JSON，也可能
//! 是一段话，消费方自己判断怎么解。
//!
//! ## 结构化字段要显式渲染
//!
//! 发布方上报的 `labels` / `annotations` **不在任务里**：它们是模板的输入，
//! 不是投递的产物。大屏要坐标，就在内容模板里把 `lon` / `lat` 渲染进
//! `content` 的 JSON，只留在 `msg_delivery` 的快照里给排查用。
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

    /// 已渲染的标题（`msg_delivery.title`）
    ///
    /// 一定非空：`msg_message.title_template` 有值就用模板，没有就用发布方
    /// 给的 `title`，两边都空的消息在 `msg-svr` 就被拒了。
    pub title: String,
    /// 已渲染的内容（`msg_delivery.content`）
    ///
    /// 同 [`Self::title`]，一定非空。格式不限定：要下发的结构化数据（坐标、
    /// 楼层这类）由内容模板渲染成 JSON 放这里。
    pub content: String,
    /// 附加内容（`msg_delivery.ext`）
    ///
    /// 可选，格式不做约定：模板渲染出的文本原样带下去，消费方自己判断是
    /// JSON 还是人话。没有就整个字段不出现。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ext: Option<String>,
    /// 消息创建时间戳（毫秒，`msg_delivery.create_ms`）
    pub create_ms: u64,

    /// 投递目标类别 ID（`msg_delivery_target.target_category_id`）
    pub target_category_id: u64,
    /// 目标 ID（`msg_delivery_target.target_id`）
    ///
    /// websocket 渠道用它定位长连接：大屏连接上来时上报的 `targetId` 就是这个值。
    ///
    /// 为 `None` 表示**按目标类别广播**：这条任务发给该类别下的全部目标，
    /// 由每个实例展开到自己本地的连接（大屏这类动态接入的目标没法逐个登记）。
    /// 消费方不能假定它一定有值。
    #[serde(default)]
    pub target_id: Option<u64>,
    /// 渠道 ID（`msg_delivery_channel.channel_id`）
    pub channel_id: u64,
    /// 渠道编码（`msg_channel.code`）
    ///
    /// 同时是投递主题 `msg.delivery.{code}` 的末段。
    pub channel_code: String,
}
