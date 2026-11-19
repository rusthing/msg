use robotech::macros::dao;

/// 消息目标与渠道
#[dao(
    unique_keys: [
        ("message_target_id,channel_id", "消息目标与渠道"),
    ],
)]
pub struct MsgMessageTargetChannelDao;
