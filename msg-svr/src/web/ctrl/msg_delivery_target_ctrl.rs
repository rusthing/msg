use robotech::macros::ctrl;

/// # 投递目标控制器（只读）
///
/// 目标行由消息处理链路按消息定义展开，手工增删改会与投递记录、渠道任务
/// 脱节。对外只保留查询。
#[ctrl(skip: [add, modify, save, upsert, sync, del_by_id, del_by_query_dto])]
struct MsgDeliveryTargetCtrl;
