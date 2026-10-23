use robotech::macros::dao;

/// 投递目标
#[dao(
    unique_keys: [
        ("delivery_id,target_category_id,target_id", "投递与目标类别与目标"),
    ],
    like_columns: [],
)]
pub struct MsgDeliveryTargetDao;
