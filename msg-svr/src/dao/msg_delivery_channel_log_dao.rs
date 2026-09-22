use robotech::macros::dao;

/// 投递渠道日志
#[dao(
    like_columns: [
        Column::Address,
        Column::Detail,
    ],
)]
pub struct MsgDeliveryChannelLogDao;