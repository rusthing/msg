//! # 再次投递的结果
//!
//! 一次重投会按「目标 × 渠道」拆成若干投递任务，结果要能回答两个问题：
//! **拆出几路**、**其中几路没发出去**。只回一句「已重投」等于让操作员去翻库。
//!
//! ## 三个计数都是 `u32`
//!
//! 与其它 VO 的 `u64` 不同：它们不是雪花 ID，也不需要躲 `2^53`，前端要直接参与
//! 「已提交 3 路，其中 1 路发布失败」这类文案。塞成 `U64` 字符串只会逼前端
//! `Number()` 一次，没有意义。
//!
//! 计数只到「任务已发布到渠道投递队列」为止，**不代表渠道已经送达**——真正的
//! 送达结果是渠道消费者异步回写的，去看渠道任务与渠道投递日志。

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// # 再次投递的结果
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MsgDeliveryRedeliverVo {
    /// 本次拆出的投递任务数（目标 × 渠道）
    pub task_count: u32,
    /// 成功发布到投递队列的任务数
    pub published_count: u32,
    /// 发布失败的任务数（已按失败回写渠道状态，渠道日志里能看到原因）
    pub failed_count: u32,
}
