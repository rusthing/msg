use robotech::macros::router;

/// # 投递渠道任务路由（只读）
///
/// 渠道任务由消息处理链路与「再次投递」维护，状态由消费者回写，
/// 不接受手工增删改，只暴露查询接口。
#[router(routes [
    ("/msg/delivery-channel", get(get_by_query_dto)),
    ("/msg/delivery-channel/ex", get(get_ex_by_query_dto)),
    ("/msg/delivery-channel/list", get(list_by_query_dto)),
    ("/msg/delivery-channel/count", get(count_by_query_dto)),
    ("/msg/delivery-channel/list-ex", get(list_ex_by_query_dto)),
    ("/msg/delivery-channel/page", get(page_by_query_dto)),
    ("/msg/delivery-channel/page-ex", get(page_ex_by_query_dto)),
    ("/msg/delivery-channel/search", get(search_by_condition)),
    ("/msg/delivery-channel/search/ex", get(search_ex_by_condition)),
    ("/msg/delivery-channel/search/list", get(search_list)),
    ("/msg/delivery-channel/search/list-ex", get(search_list_ex)),
    ("/msg/delivery-channel/search/page", get(search_page)),
    ("/msg/delivery-channel/search/page-ex", get(search_page_ex)),
    ("/msg/delivery-channel/ex/{id}", get(get_ex_by_id)),
    ("/msg/delivery-channel/search/ex/{id}", get(search_ex_by_id)),
    ("/msg/delivery-channel/search/{id}", get(search_by_id)),
    ("/msg/delivery-channel/{id}", get(get_by_id)),
])]
struct MsgDeliveryChannelRouter;
