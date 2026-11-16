use robotech::macros::ctrl;

/// # 渠道投递日志控制器（只读）
///
/// 一次发送尝试一条，由 `deliver_result::record` 追加。它是审计证据，
/// 允许改写就失去意义。对外只保留查询。
#[ctrl(skip: [add, modify, save, upsert, sync, del_by_id, del_by_query_dto])]
struct MsgDeliveryChannelLogCtrl;
