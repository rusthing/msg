use crate::dao::MsgDeliveryQueueDao;
use msg_api::cst::MSG_DELIVERY_TOPIC_PREFIX;
use robotech::macros::svc;
use sea_orm::{ColumnTrait, Condition};

/// 渠道
///
/// 写操作要同时刷新两份内存状态：消息缓存（`enabled` 决定还给不给这个渠道
/// 生成投递任务）和投递队列快照（主题末段就是渠道编码，禁用要停订阅）。
/// 两者都由 [`crate::wkr::on_channel_refresh`] 一起发信号。
///
/// 改 `code` 时还要把投递队列的主题一起改掉，见 [`MsgChannelSvc::before_modify`]。
#[svc(
    before_modify = Self::before_modify,
    after_write = crate::wkr::on_channel_refresh
)]
pub struct MsgChannelSvc;

impl MsgChannelSvc {
    /// # 改编码时同步改投递队列的主题
    ///
    /// 投递队列**不存 `channel_id`**，渠道信息只在主题里：`msg.delivery.{渠道编码}`
    /// 的末段就是 `msg_channel.code`。所以改编码而不同步主题，队列就指向一个不存在
    /// 的渠道——重载时被跳过、打一条 warn，表现是「消息突然不发了」，而且没有任何
    /// 写操作报错。这里把两处改动放进同一个事务：要么都成，要么都回滚。
    ///
    /// ## 为什么直接走 DAO，而不用 `MsgDeliveryQueueSvc::modify`
    ///
    /// 后者的 `before_modify` 会校验「主题里的渠道必须存在」。本钩子运行时渠道行
    /// 还没更新，新编码在库里的渠道仍是旧编码，校验必然失败。级联写属于内部改写，
    /// 新主题由新编码拼出来，不需要再过一遍格式/存在性校验。
    ///
    /// ## 只改主题里带旧编码的那条
    ///
    /// 一个渠道配一条队列，条件就是旧主题全等。若新主题已被别的队列占用，
    /// `AK_CODE_MSG_DELIVERY_QUEUE` 会在 update 时报重复键，事务整体回滚。
    async fn before_modify<C: ConnectionTrait>(
        modify_dto: MsgChannelModifyDto,
        db: &C,
    ) -> Result<MsgChannelModifyDto, SvcError> {
        // 没改 code 就没什么可级联的
        let (Some(id), Some(new_code)) = (modify_dto.id, modify_dto.code.clone()) else {
            return Ok(modify_dto);
        };

        // 记录不存在时交给后面的 update 报 NotFound，这里不抢它的错
        let Some(old) = Self::get_by_id(id, Some(db)).await?.extra else {
            return Ok(modify_dto);
        };
        if old.code == new_code {
            return Ok(modify_dto);
        }

        let old_topic = format!("{MSG_DELIVERY_TOPIC_PREFIX}{}", old.code);
        let new_topic = format!("{MSG_DELIVERY_TOPIC_PREFIX}{new_code}");

        let rows: Vec<crate::mo::msg_delivery_queue::Model> = MsgDeliveryQueueDao::list_by_condition(
            Condition::all().add(crate::mo::msg_delivery_queue::Column::Code.eq(old_topic)),
            &None,
            db,
        )
        .await?;

        for model in rows {
            let mut active: crate::mo::msg_delivery_queue::ActiveModel = model.into();
            active.code = sea_orm::ActiveValue::Set(new_topic.clone());
            MsgDeliveryQueueDao::update(active, db).await?;
        }

        Ok(modify_dto)
    }
}
