//! # 死信服务
//!
//! 死信只读，读写入口只有「标记已处理」这一个动作（[`MsgDeadLetterSvc::mark_handled`]）。
//! 死信行由 `dlq.{原主题}` 的消费者落库（见 `sub/dead_letter.rs`），
//! 原始负载与转入原因都要保留原样；`handled` 只表示「人已经看过并处理」。

use crate::dto::MsgDeadLetterHandleDto;
use robotech::db::get_db_conn;
use robotech::macros::svc;

// `Ro` / `U64` / `SvcError` / `MsgDeadLetterVo` / `MsgDeadLetterModifyDto` 由
// `#[svc]` 宏在本模块内注入，这里不再 `use`。
#[svc]
pub struct MsgDeadLetterSvc;

impl MsgDeadLetterSvc {
    /// # 标记死信已处理
    ///
    /// 只写 `handled = true`，其余列原样保留；重复调用结果相同。
    /// 操作员写进审计列，方便回答「这条是谁清的」。
    ///
    /// ## 参数
    ///
    /// - `dto`: 只含死信 ID
    /// - `operator_id`: 当前操作员 ID（来自网关注入的 `USER_ID`）
    pub async fn mark_handled(
        dto: MsgDeadLetterHandleDto,
        operator_id: u64,
    ) -> Result<Ro<MsgDeadLetterVo>, SvcError> {
        let db = get_db_conn()?;
        let modify = MsgDeadLetterModifyDto {
            id: Some(dto.id),
            handled: Some(true),
            _current_user_id: U64(operator_id),
            ..Default::default()
        };
        MsgDeadLetterSvc::modify(modify, Some(db.as_ref())).await
    }
}
