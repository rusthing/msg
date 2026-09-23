use crate::mo::msg_message_queue;
use robotech::macros::dao;

/// 消息
#[dao(
    unique_keys: [
        ("code", "编码"),
        ("name", "名称"),
    ],
    like_columns: [
        Column::Code,
        Column::Name,
        Column::TitleTemplate,
        Column::ContentTemplate,
        Column::Remark,
    ],
    related_table: ["msg_message_queue"],
)]
pub struct MsgMessageDao;