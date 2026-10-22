use robotech::macros::dao;

/// 消息类别
#[dao(
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgMessageCategoryDao;