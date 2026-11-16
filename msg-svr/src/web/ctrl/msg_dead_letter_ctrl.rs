use robotech::macros::ctrl;

/// # 死信控制器（只读 + 标记已处理）
///
/// 死信是 `dlq.{原主题}` 落库的原始证据，`payload` / `reason` / `delivered`
/// 都必须保持原样，所以 CRUD 的写操作全部跳过。唯一需要人的动作是排查完后
/// 标记「已处理」——它只改 `handled`，见下面的 `handle`。
#[ctrl(skip: [add, modify, save, upsert, sync, del_by_id, del_by_query_dto])]
struct MsgDeadLetterCtrl;

use crate::dto::MsgDeadLetterHandleDto;

/// # 标记死信已处理
///
/// 只把 `handled` 置为 `true`，原始负载与转入原因不动；重复调用结果相同。
///
/// ## 请求头
///
/// 需要 `USER_ID`：操作员会写进死信的更新人，便于追溯是谁清理的。
#[utoipa::path(
    post,
    path = "/msg/dead-letter/handle",
    request_body = MsgDeadLetterHandleDto,
    responses((status = OK, body = Ro<MsgDeadLetterVo>))
)]
#[debug_handler]
#[log_call]
pub async fn handle(
    headers: HeaderMap,
    Json(dto): Json<MsgDeadLetterHandleDto>,
) -> Result<Json<Ro<MsgDeadLetterVo>>, CtrlError> {
    let operator_id = get_current_user_id(&headers)?.0;
    Ok(Json(
        MsgDeadLetterSvc::mark_handled(dto, operator_id).await?,
    ))
}
