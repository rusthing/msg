use robotech::macros::ctrl;

/// # 投递渠道任务控制器（只读）
///
/// 一行 = 一个目标 × 一个渠道，由消息处理链路与「再次投递」维护；状态回写
/// 也由消费者负责。手工增删改会让状态汇总失真。对外只保留查询。
#[ctrl(skip: [add, modify, save, upsert, sync, del_by_id, del_by_query_dto])]
struct MsgDeliveryChannelCtrl;
