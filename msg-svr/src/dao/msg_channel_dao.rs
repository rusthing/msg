use robotech::macros::dao;

/// 渠道
#[dao(
    like_columns: [
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgChannelDao;