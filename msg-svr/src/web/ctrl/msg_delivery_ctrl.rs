use robotech::macros::ctrl;

#[ctrl]
struct MsgDeliveryCtrl;

// ============================================================================
// 投递记录的两个写动作：CRUD 之外的「再次投递」
//
// 投递记录由消息处理链路自动生成（`msg_delivery`），运营一般只读不写。
// 唯一需要人的地方是「这条没送到，再发一遍」——渠道挂过、大屏重启过、
// 当时那一路失败了。所以这个控制器只加一个动作，不开放「手工新增投递」：
// 新增一条投递记录等于伪造一次业务触发，会把幂等键和延迟统计一起搞乱。
// ============================================================================

use crate::dto::MsgDeliveryRedeliverDto;
use crate::vo::MsgDeliveryRedeliverVo;

/// # 再次投递
///
/// 把一条已有的投递记录按它当时的「目标 × 渠道」重新发一遍：状态回到投递中，
/// 重新发布投递任务，发送结果仍由渠道消费者异步回写。
///
/// ## 为什么不新建一条投递记录
///
/// 投递记录的唯一键 `(event_code, business_id, business_trigger_ms)` 里
/// `business_trigger_ms` 是业务时刻。重投要么撞唯一键，要么伪造一个当前时间——
/// 后者会让端到端延迟与对账全部失真。所以重投写回原记录，每次尝试单独记一行
/// 渠道投递日志（`detail` 前缀「手工重投」）。
///
/// ## 返回值
///
/// 拆出了几路、其中几路没发出去。计数只到「已发布到渠道投递队列」，**不代表
/// 已送达**：送达结果是异步回写的，去看渠道任务与渠道投递日志。
///
/// ## 请求头
///
/// 需要 `USER_ID`：重投是人的动作，操作员会写进三级投递记录的更新人。
#[utoipa::path(
    post,
    path = "/msg/delivery/redeliver",
    request_body = MsgDeliveryRedeliverDto,
    responses((status = OK, body = Ro<MsgDeliveryRedeliverVo>))
)]
#[debug_handler]
#[log_call]
pub async fn redeliver(
    headers: HeaderMap,
    Json(dto): Json<MsgDeliveryRedeliverDto>,
) -> Result<Json<Ro<MsgDeliveryRedeliverVo>>, CtrlError> {
    let operator_id = get_current_user_id(&headers)?.0;
    Ok(Json(MsgDeliverySvc::redeliver(dto, operator_id).await?))
}
