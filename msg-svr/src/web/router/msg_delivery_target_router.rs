use robotech::macros::router;

/// # 投递目标路由（只读）
///
/// 目标行由消息处理链路展开，不接受增删改，只暴露查询接口。
#[router(routes [
    ("/msg/delivery-target", get(get_by_query_dto)),
    ("/msg/delivery-target/ex", get(get_ex_by_query_dto)),
    ("/msg/delivery-target/list", get(list_by_query_dto)),
    ("/msg/delivery-target/list-ex", get(list_ex_by_query_dto)),
    ("/msg/delivery-target/page", get(page_by_query_dto)),
    ("/msg/delivery-target/page-ex", get(page_ex_by_query_dto)),
    ("/msg/delivery-target/search", get(search_by_condition)),
    ("/msg/delivery-target/search/ex", get(search_ex_by_condition)),
    ("/msg/delivery-target/search/list", get(search_list)),
    ("/msg/delivery-target/search/list-ex", get(search_list_ex)),
    ("/msg/delivery-target/search/page", get(search_page)),
    ("/msg/delivery-target/search/page-ex", get(search_page_ex)),
    ("/msg/delivery-target/ex/{id}", get(get_ex_by_id)),
    ("/msg/delivery-target/search/ex/{id}", get(search_ex_by_id)),
    ("/msg/delivery-target/search/{id}", get(search_by_id)),
    ("/msg/delivery-target/{id}", get(get_by_id)),
])]
struct MsgDeliveryTargetRouter;
