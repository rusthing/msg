use robotech::macros::dao;

/// 消息级别
#[dao(
    unique_keys: [
        ("code", "编码"),
        ("name", "名称"),
    ],
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgMessageLevelDao;