use robotech::macros::router;

/// # 死信路由（只读 + 标记已处理）
///
/// 死信不接受增删改，只暴露查询与「标记已处理」：
///
/// | 方法与路径 | 用途 |
/// |------------|------|
/// | `GET /msg/dead-letter` 等 | 查询（详情 / 条件 / 列表 / 分页 / 检索） |
/// | `POST /msg/dead-letter/handle` | 标记已处理（只改 `handled`） |
///
/// 静态路径排在 `{id}` 之前：`GET /msg/dead-letter/{id}` 是详情接口，
/// 不能被动作名吃掉。
#[router(routes [
    ("/msg/dead-letter", get(get_by_query_dto)),
    ("/msg/dead-letter/ex", get(get_ex_by_query_dto)),
    ("/msg/dead-letter/list", get(list_by_query_dto)),
    ("/msg/dead-letter/count", get(count_by_query_dto)),
    ("/msg/dead-letter/list-ex", get(list_ex_by_query_dto)),
    ("/msg/dead-letter/page", get(page_by_query_dto)),
    ("/msg/dead-letter/page-ex", get(page_ex_by_query_dto)),
    ("/msg/dead-letter/search", get(search_by_condition)),
    ("/msg/dead-letter/search/ex", get(search_ex_by_condition)),
    ("/msg/dead-letter/search/list", get(search_list)),
    ("/msg/dead-letter/search/list-ex", get(search_list_ex)),
    ("/msg/dead-letter/search/page", get(search_page)),
    ("/msg/dead-letter/search/page-ex", get(search_page_ex)),
    ("/msg/dead-letter/handle", post(handle)),
    ("/msg/dead-letter/ex/{id}", get(get_ex_by_id)),
    ("/msg/dead-letter/search/ex/{id}", get(search_ex_by_id)),
    ("/msg/dead-letter/search/{id}", get(search_by_id)),
    ("/msg/dead-letter/{id}", get(get_by_id)),
])]
struct MsgDeadLetterRouter;
