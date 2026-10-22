use robotech::macros::dao;

/// 目标类别与渠道
#[dao(
    unique_keys: [
        ("channel_id,target_category_id", "渠道与目标类别"),
    ],
)]
pub struct MsgTargetCategoryChannelDao;