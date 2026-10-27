//! # 投递队列服务
//!
//! 投递队列把「渠道」和「投递主题」绑在一起，但**不存渠道 ID**：主题固定为
//! `msg.delivery.{渠道编码}`，末段就是 `msg_channel.code`，运行时由
//! [`delivery_topic_channel_code`] 反解（见 [`crate::wkr::sub`]）。
//!
//! ## 为什么写入时要校验
//!
//! 去掉外键之后，「主题指向哪个渠道」这件事只在写入口守得住。主题写错不会有
//! 任何报错：重载时认不出渠道，打一条 warn 就跳过——运营看到的是「队列配好了
//! 但消息一直不出去」。所以新增/修改前先校验前缀、末段字符集与渠道是否存在，
//! 把这类错误挡在写库之前，语义上替代原来那条外键约束。
//!
//! 渠道**禁用不挡写入**：队列可以先配好，等渠道启用时才真正订阅。禁用只在
//! [`crate::wkr::sub::reload_delivery_queues`] 里生效。

use crate::dto::MsgChannelQueryDto;
use crate::svc::MsgChannelSvc;
use msg_api::cst::{MSG_DELIVERY_TOPIC_PREFIX, delivery_topic_channel_code};
use robotech::macros::svc;
use validator::ValidationError;

#[svc(
    before_add = Self::before_add,
    before_modify = Self::before_modify,
    after_write = crate::wkr::on_delivery_queue_refresh
)]
pub struct MsgDeliveryQueueSvc;

// SvcError 由框架定义且体积较大，而钩子/服务方法的返回类型必须与宏约定一致，
// 无法改成 Box 包裹，这里统一放行该 lint
#[allow(clippy::result_large_err)]
impl MsgDeliveryQueueSvc {
    /// # 新增前：校验主题能对上一个真实渠道
    ///
    /// `db` 是本次插入所在的事务，查渠道走同一个事务，校验与写入不会看到
    /// 两个不同快照。
    async fn before_add<C: ConnectionTrait>(
        add_dto: MsgDeliveryQueueAddDto,
        db: &C,
    ) -> Result<MsgDeliveryQueueAddDto, SvcError> {
        if let Some(code) = add_dto.code.as_deref() {
            check_delivery_topic(code, db).await?;
        }
        Ok(add_dto)
    }

    /// # 修改前：只校验本次真正改了主题的情况
    async fn before_modify<C: ConnectionTrait>(
        modify_dto: MsgDeliveryQueueModifyDto,
        db: &C,
    ) -> Result<MsgDeliveryQueueModifyDto, SvcError> {
        if let Some(code) = modify_dto.code.as_deref() {
            check_delivery_topic(code, db).await?;
        }
        Ok(modify_dto)
    }
}

/// # 校验投递主题
///
/// 三项都要过，缺一项运行时就会变成「配了但永远不投」：
///
/// 1. 前缀必须是 [`MSG_DELIVERY_TOPIC_PREFIX`]，否则反解不出渠道；
/// 2. 末段只能是 `[A-Za-z0-9_-]`——它就是 NATS subject 的一个 token，
///    带 `.` / `*` / `>` 会把主题拆成多级甚至变成通配；
/// 3. 渠道必须真的存在（`msg_channel.code`），否则没有发送器认领。
///
/// 渠道禁用不影响这里：队列可以先配着，等启用再订阅。
async fn check_delivery_topic<C: ConnectionTrait>(
    code: &str,
    db: &C,
) -> Result<(), SvcError> {
    let Some(channel_code) = delivery_topic_channel_code(code) else {
        return Err(validation_error(
            "投递主题不合约定",
            format!(
                "主题必须是 {MSG_DELIVERY_TOPIC_PREFIX}{{渠道编码}}，例如 {MSG_DELIVERY_TOPIC_PREFIX}websocket"
            ),
        ));
    };

    if !channel_code
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(validation_error(
            "渠道编码含非法字符",
            format!(
                "主题末段 {channel_code} 只能包含字母、数字、- 和 _（它是 NATS subject 的一个 token）"
            ),
        ));
    }

    let exists = !MsgChannelSvc::list_by_query_dto(
        MsgChannelQueryDto {
            code: Some(channel_code.to_string()),
            ..Default::default()
        },
        Some(db),
    )
    .await?
    .extra
    .unwrap_or_default()
    .is_empty();

    if !exists {
        return Err(validation_error(
            "渠道不存在",
            format!("没有 code = {channel_code} 的渠道，先建渠道再配投递队列"),
        ));
    }

    Ok(())
}

/// 构造一个参数校验错误
fn validation_error(code: &'static str, message: String) -> SvcError {
    SvcError::Validation(ValidationError::new(code).with_message(message.into()))
}
