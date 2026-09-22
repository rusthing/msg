use robotech::macros::dao;

/// 投递渠道
#[dao(
    unique_keys: [
        ("deliver_target_id,channel_id", "投递目标与渠道"),
    ],
    like_columns: [
        Column::Address,
    ],
)]
pub struct MsgDeliveryChannelDao;