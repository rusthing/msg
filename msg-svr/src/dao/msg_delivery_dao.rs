use robotech::macros::dao;

/// 投递
///
/// 幂等键是「事件编码 + 业务 ID + 业务触发时间戳」三者组合，不是单独的
/// `business_id`：同一条业务消息可能在不同时间点触发多次投递。
#[dao(
    unique_keys: [
        ("event_code,business_id,business_trigger_ms", "事件编码与业务ID与业务触发时间戳"),
    ],
    like_columns: [
        Column::Title,
        Column::Content,
        Column::Remark,
    ],
)]
pub struct MsgDeliveryDao;