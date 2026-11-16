use robotech::macros::api_doc;

/// 死信只读；`handle` 是唯一写动作。
#[api_doc(
    get_by_id,
    get_ex_by_id,
    get_by_query_dto,
    get_ex_by_query_dto,
    list_by_query_dto,
    list_ex_by_query_dto,
    page_by_query_dto,
    page_ex_by_query_dto,
    search_by_id,
    search_by_condition,
    search_list,
    search_page,
    search_ex_by_id,
    search_ex_by_condition,
    search_list_ex,
    search_page_ex,
    handle
)]
pub struct MsgDeadLetterApiDoc;
