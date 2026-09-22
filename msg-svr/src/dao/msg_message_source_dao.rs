use robotech::macros::dao;

/// 消息来源
#[dao(
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgMessageSourceDao;