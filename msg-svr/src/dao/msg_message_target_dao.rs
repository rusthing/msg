use robotech::macros::dao;

/// 消息目标
#[dao(
    unique_keys: [
        ("message_id,target_category_id,target_id", "消息与目标类别与目标"),
    ],
    like_columns: [],
)]
pub struct MsgMessageTargetDao;