use robotech::macros::dao;

/// 渠道
#[dao(
    unique_keys: [
        ("code", "编码"),
    ],
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgChannelDao;