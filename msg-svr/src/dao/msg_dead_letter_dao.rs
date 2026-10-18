use robotech::macros::dao;

/// 死信消息
#[dao(
    like_columns: [
        Column::Subject,
        Column::Reason,
        Column::Remark,
    ],
)]
pub struct MsgDeadLetterDao;
