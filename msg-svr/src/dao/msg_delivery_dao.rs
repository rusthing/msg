use robotech::macros::dao;

/// 投递
#[dao(
    unique_keys: [
        ("business_id", "业务ID"),
    ],
    like_columns: [
        Column::Title,
        Column::Content,
        Column::Remark,
    ],
)]
pub struct MsgDeliveryDao;