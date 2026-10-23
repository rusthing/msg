use robotech::macros::dao;

/// 投递队列
#[dao(
    unique_keys: [
        ("code", "编码"),
        ("name", "名称"),
        ("channel_id", "渠道"),
    ],
    like_columns: [
        Column::Code,
        Column::Name,
        Column::Remark,
    ],
)]
pub struct MsgDeliveryQueueDao;
