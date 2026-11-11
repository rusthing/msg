//! # 再次投递的请求体
//!
//! 不是 CRUD 的载体：投递记录只允许「再发一遍」，其余列一律由服务端按原记录
//! 取，所以请求里只有一个投递 ID。
//!
//! ## 为什么不让前端传目标 / 渠道
//!
//! 重投的语义是「把这条投递当时发过的那几路再发一遍」。目标与渠道已经在
//! `msg_delivery_target` / `msg_delivery_channel` 里落好了，让调用方重传一份
//! 就等于给它机会改掉「这次重投到底发给谁」——那已经不是重投，是新投递，应该
//! 走上报入口重新生成一条投递记录。
//!
//! `u64` 字段用框架的 [`U64`]：线上是字符串，19 位雪花 ID 不经过 JS 的 number。

use robotech::api::U64;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// # 再次投递
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgDeliveryRedeliverDto {
    /// 投递记录 ID（`msg_delivery.id`）
    pub id: U64,
}
