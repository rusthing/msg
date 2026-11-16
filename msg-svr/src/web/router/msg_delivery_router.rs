use robotech::macros::router;

/// # 投递记录路由（只读 + 再次投递）
///
/// 投递记录不接受增删改，只暴露查询与「再次投递」：
///
/// | 方法与路径 | 用途 |
/// |------------|------|
/// | `GET /msg/delivery` 等 | 查询（详情 / 条件 / 列表 / 分页 / 检索） |
/// | `POST /msg/delivery/redeliver` | 再次投递（按原目标 × 渠道重发一遍） |
///
/// 静态路径排在 `{id}` 之前：`GET /msg/delivery/{id}` 是详情接口，
/// 不能被动作名吃掉。
#[router(routes [
    ("/msg/delivery", get(get_by_query_dto)),
    ("/msg/delivery/ex", get(get_ex_by_query_dto)),
    ("/msg/delivery/list", get(list_by_query_dto)),
    ("/msg/delivery/list-ex", get(list_ex_by_query_dto)),
    ("/msg/delivery/page", get(page_by_query_dto)),
    ("/msg/delivery/page-ex", get(page_ex_by_query_dto)),
    ("/msg/delivery/search", get(search_by_condition)),
    ("/msg/delivery/search/ex", get(search_ex_by_condition)),
    ("/msg/delivery/search/list", get(search_list)),
    ("/msg/delivery/search/list-ex", get(search_list_ex)),
    ("/msg/delivery/search/page", get(search_page)),
    ("/msg/delivery/search/page-ex", get(search_page_ex)),
    ("/msg/delivery/redeliver", post(redeliver)),
    ("/msg/delivery/ex/{id}", get(get_ex_by_id)),
    ("/msg/delivery/search/ex/{id}", get(search_ex_by_id)),
    ("/msg/delivery/search/{id}", get(search_by_id)),
    ("/msg/delivery/{id}", get(get_by_id)),
])]
struct MsgDeliveryRouter;
