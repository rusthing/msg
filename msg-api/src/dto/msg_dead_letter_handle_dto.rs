//! # 死信「已处理」的请求体
//!
//! 死信只读，唯一需要人的动作是「排查完，标记已处理」。请求里只有一个主键：
//! `handled` 固定由服务端写 `true`，其它列（原始负载、转入原因）是落库时的
//! 证据，不允许顺带修改。
//!
//! `u64` 字段用框架的 [`U64`]：线上是字符串，19 位雪花 ID 不经过 JS 的 number。

use robotech::api::U64;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// # 标记死信已处理
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgDeadLetterHandleDto {
    /// 死信 ID（`msg_dead_letter.id`）
    pub id: U64,
}
