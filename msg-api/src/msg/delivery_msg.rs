//! # 投递任务
//!
//! `msg-svr` 渲染完消息后，按「目标 × 渠道」拆任务，发布到
//! `msg.delivery.{渠道编码}`，由该渠道的发送方消费。
//!
//! ## 任务里带什么
//!
//! 一条投递任务就是「这条消息该下发的东西」，分三块：
//!
//! - **渲染结果**：[`DeliveryMsg::title`] / [`DeliveryMsg::content`]，模板套完变量
//!   的可读文案，渠道可以直接发；
//! - **原始信息**：[`DeliveryMsg::labels`] / [`DeliveryMsg::annotations`]，发布方给的
//!   结构化字段（坐标、层级、设备号这类），模板拿它当变量，接收方也拿它做业务处理
//!   （大屏按 `lon` / `lat` 落点、按 `levelId` 切楼层）；
//! - **消息身份**：`event_code` / `message_id` / 类别与来源的编码名称，
//!   接收方不查库也能知道这条消息是谁、属于哪一类。
//!
//! 模板只负责把变量拼成文案，**不吞掉输入**：渲染出标题不等于原始信息就没用了，
//! 同一份 `labels` / `annotations` 必须原样跟着任务走。
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
use std::collections::HashMap;
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
    /// 消息名称（`msg_message.name`）
    pub message_name: String,
    /// 消息类别编码（`msg_message_category.code`）
    pub category_code: String,
    /// 消息类别名称（`msg_message_category.name`）
    ///
    /// 渠道可以直接拿它做文案前缀或分组，不必再回查类别表。
    pub category_name: String,
    /// 事件来源编码（`msg_event_source.code`）
    pub source_code: String,
    /// 事件来源名称（`msg_event_source.name`）
    pub source_name: String,

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
    /// 与来源消息的 `business_id` 同值，发布方没给就是 `None`。
    /// 持久化消息必然有值（缺了在幂等阶段就被拒）；非持久化消息可有可无，
    /// 有的话同样下发（定位消息就带着绑定目标 ID），只是不参与幂等。
    #[serde(default)]
    pub business_id: Option<u64>,
    /// 业务触发时间戳（毫秒）
    ///
    /// 同 [`Self::business_id`]：非持久化消息可为 `None`。
    #[serde(default)]
    pub business_trigger_ms: Option<u64>,

    /// 结构化标签（发布方原样上报的 `labels`）
    ///
    /// 既是模板变量，也是投递内容的一部分：接收方按 key 取业务字段做联动
    /// （`device_id` / `level_id` 这类），不依赖标题里有没有拼出来。
    #[serde(default)]
    #[builder(default)]
    pub labels: HashMap<String, String>,
    /// 可读标注（发布方原样上报的 `annotations`）
    ///
    /// 同 [`Self::labels`]：模板变量与投递内容两用。坐标、楼层、绑定对象
    /// 这些展示侧要用的字段都在这里，大屏拿它落点、画图标。
    #[serde(default)]
    #[builder(default)]
    pub annotations: HashMap<String, String>,

    /// 已渲染的标题
    ///
    /// 模板把 `labels` / `annotations` 的变量注入后得到的结果，渠道可以直接用；
    /// 与 [`Self::labels`] / [`Self::annotations`] 并存，不是替代关系。
    pub title: String,
    /// 已渲染的内容
    ///
    /// 同 [`Self::title`]。
    pub content: String,
    /// 消息生成时间戳（毫秒）
    pub created_ms: u64,
}
