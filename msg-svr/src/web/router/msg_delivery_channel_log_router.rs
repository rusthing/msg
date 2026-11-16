use robotech::macros::router;

/// # 渠道投递日志路由（只读）
///
/// 一次发送尝试一条，是审计证据，不接受增删改，只暴露查询接口。
#[router(routes [
    ("/msg/delivery-channel-log", get(get_by_query_dto)),
    ("/msg/delivery-channel-log/ex", get(get_ex_by_query_dto)),
    ("/msg/delivery-channel-log/list", get(list_by_query_dto)),
    ("/msg/delivery-channel-log/list-ex", get(list_ex_by_query_dto)),
    ("/msg/delivery-channel-log/page", get(page_by_query_dto)),
    ("/msg/delivery-channel-log/page-ex", get(page_ex_by_query_dto)),
    ("/msg/delivery-channel-log/search", get(search_by_condition)),
    ("/msg/delivery-channel-log/search/ex", get(search_ex_by_condition)),
    ("/msg/delivery-channel-log/search/list", get(search_list)),
    ("/msg/delivery-channel-log/search/list-ex", get(search_list_ex)),
    ("/msg/delivery-channel-log/search/page", get(search_page)),
    ("/msg/delivery-channel-log/search/page-ex", get(search_page_ex)),
    ("/msg/delivery-channel-log/ex/{id}", get(get_ex_by_id)),
    ("/msg/delivery-channel-log/search/ex/{id}", get(search_ex_by_id)),
    ("/msg/delivery-channel-log/search/{id}", get(search_by_id)),
    ("/msg/delivery-channel-log/{id}", get(get_by_id)),
])]
struct MsgDeliveryChannelLogRouter;
