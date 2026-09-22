use robotech::macros::dao;

/// 消息渠道
#[dao(
    unique_keys: [
        ("message_id,channel_id", "消息与渠道"),
    ],
    like_columns: [],
)]
pub struct MsgMessageChannelDao;