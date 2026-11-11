//! # 投递服务
//!
//! 除 `#[svc]` 生成的 CRUD 之外，这里只有一件事：**再次投递**
//! （[`MsgDeliverySvc::redeliver`]）。
//!
//! ## 为什么不是「新建一条投递记录」
//!
//! 投递记录的唯一键是 `(event_code, business_id, business_trigger_ms)`，其中
//! `business_trigger_ms` 是**业务时刻**，不是服务端时间。重投时造一条新记录，
//! 要么沿用原值被唯一键挡住，要么拿 `now()` 顶上——那就是伪造业务时间，之后
//! 所有按业务时刻算的延迟、对账全部失真。
//!
//! 所以重投**写回原记录**：把它当时拆出来的「目标 × 渠道」状态重置为投递中，
//! 用库里已有的快照（标题 / 内容 / ext / 三张表的行 ID）重新组装投递任务发出去。
//! 每次尝试在 `msg_delivery_channel_log` 里单独留一行（那张表没有唯一键，本来就
//! 是按尝试累加的），审计不丢。
//!
//! ## 顺序：先改状态，再发任务
//!
//! 反过来会翻车：任务发出去后渠道消费者可能几百毫秒内就回写成功，随后我们再
//! 把状态改回「投递中」，这一路就永远停在投递中——没有第二次回写来收尾。
//!
//! ## 与首次投递的差别
//!
//! - 目标与渠道**照旧**，不重新读消息定义的目标配置：重投的语义就是「把发过的
//!   那几路再发一遍」，换了接收方那是另一次投递；
//! - 渠道编码要现查（`msg_channel.code`）：任务里必须带编码，消费方按它认领；
//! - 渠道被禁用或没配投递队列时，`publish_task` 会报错，那就按发布失败回写，
//!   不影响同一投递的其它渠道。

use crate::dto::{
    MsgChannelQueryDto, MsgDeliveryChannelModifyDto, MsgDeliveryChannelQueryDto,
    MsgDeliveryModifyDto, MsgDeliveryQueryDto, MsgDeliveryRedeliverDto,
    MsgDeliveryTargetModifyDto, MsgDeliveryTargetQueryDto, MsgMessageQueryDto,
    MsgTargetCategoryQueryDto,
};
use crate::svc::{
    MsgChannelSvc, MsgDeliveryChannelSvc, MsgDeliveryTargetSvc, MsgMessageSvc,
    MsgTargetCategorySvc,
};
use crate::utils::server_now_ms;
use crate::vo::MsgDeliveryRedeliverVo;
use crate::wkr::channel::SendOutcome;
use crate::wkr::{deliver_result, publish_task};
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::msg::DeliveryMsg;
use robotech::db::get_db_conn;
use robotech::macros::svc;
use sea_orm::DatabaseConnection;
use tracing::{error, info};
use validator::ValidationError;

// 注意：`Ro` / `U64` / `SvcError` / `MsgDeliveryVo` 由 `#[svc]` 宏在本模块内注入，
// 这里再 `use` 一次会报「the name is defined multiple times」。
#[svc]
pub struct MsgDeliverySvc;

/// # 一路待重投的任务
///
/// 「目标 × 渠道」的纯数据。先把要发的读齐、状态改完，再统一发任务：
/// 中途查不到渠道编码就整批放弃，不会出现「一半改了状态、一半没发」。
struct RedeliverTask {
    /// `msg_delivery_target.id`
    delivery_target_id: u64,
    /// `msg_delivery_target.target_category_id`
    target_category_id: u64,
    /// `msg_target_category.code`（消费方按它认领）
    target_category_code: String,
    /// `msg_delivery_target.target_id`，空串表示按类别广播
    target_id: String,
    /// `msg_delivery_channel.id`
    delivery_channel_id: u64,
    /// `msg_delivery_channel.channel_id`
    channel_id: u64,
    /// `msg_channel.code`（投递主题末段，消费方按它认领）
    channel_code: String,
}

// `SvcError` 由框架定义且体积较大，而服务方法的返回类型必须与宏约定一致，
// 无法改成 Box 包裹，这里统一放行该 lint
#[allow(clippy::result_large_err)]
impl MsgDeliverySvc {
    /// # 再次投递
    ///
    /// 按投递记录里已经落好的「目标 × 渠道」重新发一遍，返回拆出了几路、
    /// 其中几路没发出去。
    ///
    /// ## 拒绝的情形
    ///
    /// - 投递记录不存在；
    /// - 消息定义已被删除（拿不到「是否允许模拟」，也就不该替它决定）；
    /// - 模拟投递、而消息定义已关闭「允许模拟投递」——与 `process_message` 的
    ///   模拟门控同一口径，不能靠手工操作绕开；
    /// - 一条目标记录都没有：没有接收方，发了也是零任务。
    ///
    /// ## 允许重复点
    ///
    /// 不加锁、也不做「投递中禁止重投」的判断：广播目标在没有任何连接时状态会
    /// 一直停在投递中（见 [`deliver_result`] 模块注释），那恰恰是最需要重投的
    /// 场景。连点两次的结果是多收两条，比「点了没反应」好。
    pub async fn redeliver(
        dto: MsgDeliveryRedeliverDto,
        operator_id: u64,
    ) -> Result<Ro<MsgDeliveryRedeliverVo>, SvcError> {
        let db = get_db_conn()?;
        let delivery = get_delivery(dto.id, db.as_ref()).await?;
        ensure_sim_allowed(&delivery, db.as_ref()).await?;

        let tasks = plan_tasks(&delivery, db.as_ref()).await?;
        if tasks.is_empty() {
            return Err(validation_error(
                "投递没有目标",
                format!(
                    "投递 {} 没有可重投的目标记录（msg_delivery_target 为空）",
                    delivery.id
                ),
            ));
        }

        // ── 1. 状态回到「投递中」（必须在发任务之前，理由见模块注释）──
        reset_status(&delivery, &tasks, operator_id, db.as_ref()).await?;

        // ── 2. 逐路重发 ──
        let task_count = tasks.len() as u32;
        let mut published_count = 0u32;
        let mut failed_count = 0u32;

        for item in &tasks {
            let task = build_task(&delivery, item);
            match publish_task(&task).await {
                Ok(()) => published_count += 1,
                Err(e) => {
                    failed_count += 1;
                    error!(
                        "再次投递发布失败: delivery_id={}, channel={}, target_id={}, error={e}",
                        delivery.id, item.channel_code, item.target_id
                    );
                    // 发不出去就不会有发送结果回来，就地记失败，否则这一路会一直挂在
                    // 「投递中」。接收时间用这次发布尝试的时刻——任务压根没到消费端，
                    // 这是最接近的事实。`redeliver = true` 会让日志的 detail 带上
                    // 「手工重投」，见 `deliver_result::record`。
                    deliver_result::record(
                        &task,
                        &SendOutcome::Failed(format!("再次投递时发布任务失败: {e}")),
                        server_now_ms(),
                    )
                    .await;
                }
            }
        }

        info!(
            "再次投递完成: delivery_id={}, operator_id={operator_id}, task={task_count}, \
             published={published_count}, failed={failed_count}",
            delivery.id
        );

        Ok(Ro::success(format!("已重投 {published_count}/{task_count} 路")).extra(Some(
            MsgDeliveryRedeliverVo {
                task_count,
                published_count,
                failed_count,
            },
        )))
    }
}

/// # 取投递记录
///
/// 没有就用 `NotFound`：这条记录是重投的全部依据（内容快照、目标、时间戳），
/// 缺了没有任何东西可以兜底。
async fn get_delivery(id: U64, db: &DatabaseConnection) -> Result<MsgDeliveryVo, SvcError> {
    MsgDeliverySvc::get_by_query_dto(
        MsgDeliveryQueryDto {
            id: Some(id),
            ..Default::default()
        },
        Some(db),
    )
    .await?
    .extra
    .ok_or_else(|| SvcError::NotFound(format!("投递记录 {id} 不存在")))
}

/// # 模拟门控
///
/// 与 `process_message` 第 2.5 步同一口径：带模拟标记的投递，只有在消息定义
/// 打开「允许模拟投递」时才可以发。手工重投不能成为绕过这个开关的后门——
/// 否则运营关掉开关后，谁点一下重投，模拟报警就又上生产屏了。
///
/// 消息定义查不到就直接失败：拿不到 `sim_enabled` 就没法判断，替调用方默认
/// 「允许」或「拒绝」都是在猜。
async fn ensure_sim_allowed(
    delivery: &MsgDeliveryVo,
    db: &DatabaseConnection,
) -> Result<(), SvcError> {
    let message = MsgMessageSvc::get_by_query_dto(
        MsgMessageQueryDto {
            id: Some(delivery.message_id),
            ..Default::default()
        },
        Some(db),
    )
    .await?
    .extra
    .ok_or_else(|| {
        SvcError::NotFound(format!(
            "投递 {} 引用的消息定义 {} 已不存在",
            delivery.id, delivery.message_id
        ))
    })?;

    if delivery.simulated && !message.sim_enabled {
        return Err(validation_error(
            "模拟投递已关闭",
            format!(
                "投递 {} 是模拟消息，而消息定义 {} 未开启「允许模拟投递」，不能重投",
                delivery.id, message.event_code
            ),
        ));
    }

    Ok(())
}

/// # 读齐要重投的「目标 × 渠道」
///
/// 任何一路查不到目标类别或渠道，都当整条重投失败：少发一路而不报错，比失败
/// 更难发现——投递记录会显示「全部成功」，可实际上有个渠道一次都没收到。
async fn plan_tasks(
    delivery: &MsgDeliveryVo,
    db: &DatabaseConnection,
) -> Result<Vec<RedeliverTask>, SvcError> {
    let targets = MsgDeliveryTargetSvc::list_by_query_dto(
        MsgDeliveryTargetQueryDto {
            delivery_id: Some(delivery.id),
            ..Default::default()
        },
        Some(db),
    )
    .await?
    .extra
    .unwrap_or_default();

    let mut tasks = Vec::new();
    for target in &targets {
        let category = MsgTargetCategorySvc::get_by_query_dto(
            MsgTargetCategoryQueryDto {
                id: Some(target.target_category_id),
                ..Default::default()
            },
            Some(db),
        )
        .await?
        .extra
        .ok_or_else(|| {
            SvcError::NotFound(format!(
                "投递目标 {} 指向的目标类别 {} 不存在",
                target.id, target.target_category_id
            ))
        })?;

        let channels = MsgDeliveryChannelSvc::list_by_query_dto(
            MsgDeliveryChannelQueryDto {
                deliver_target_id: Some(target.id),
                ..Default::default()
            },
            Some(db),
        )
        .await?
        .extra
        .unwrap_or_default();

        for channel in &channels {
            let channel_vo = MsgChannelSvc::get_by_query_dto(
                MsgChannelQueryDto {
                    id: Some(channel.channel_id),
                    ..Default::default()
                },
                Some(db),
            )
            .await?
            .extra
            .ok_or_else(|| {
                SvcError::NotFound(format!(
                    "投递渠道任务 {} 指向的渠道 {} 不存在",
                    channel.id, channel.channel_id
                ))
            })?;

            tasks.push(RedeliverTask {
                delivery_target_id: target.id.0,
                target_category_id: target.target_category_id.0,
                target_category_code: category.code.clone(),
                target_id: target.target_id.clone(),
                delivery_channel_id: channel.id.0,
                channel_id: channel.channel_id.0,
                channel_code: channel_vo.code.clone(),
            });
        }
    }

    Ok(tasks)
}

/// # 把投递 / 目标 / 渠道三级状态重置为「投递中」
///
/// 顺序从粗到细没有讲究，但**必须在发布任务之前全部完成**：中间只要有一路
/// 先发了，消费者回来的结果就可能被后面的重置盖掉。
///
/// `address` 不动：它是上一次尝试的落点，下一次回写会覆盖它；先清空反而会让
/// 「上次发到哪」这段信息在重投失败前就丢了。
async fn reset_status(
    delivery: &MsgDeliveryVo,
    tasks: &[RedeliverTask],
    operator_id: u64,
    db: &DatabaseConnection,
) -> Result<(), SvcError> {
    let delivery_modify = MsgDeliveryModifyDto {
        id: Some(delivery.id),
        deliver_status: Some(DeliverStatus::Delivering),
        _current_user_id: U64(operator_id),
        ..Default::default()
    };
    MsgDeliverySvc::modify(delivery_modify, Some(db)).await?;

    for item in tasks {
        let target_modify = MsgDeliveryTargetModifyDto {
            id: Some(U64(item.delivery_target_id)),
            deliver_target_status: Some(DeliverTargetStatus::Delivering),
            _current_user_id: U64(operator_id),
            ..Default::default()
        };
        MsgDeliveryTargetSvc::modify(target_modify, Some(db)).await?;

        let channel_modify = MsgDeliveryChannelModifyDto {
            id: Some(U64(item.delivery_channel_id)),
            deliver_channel_status: Some(DeliverChannelStatus::Delivering),
            _current_user_id: U64(operator_id),
            ..Default::default()
        };
        MsgDeliveryChannelSvc::modify(channel_modify, Some(db)).await?;
    }

    Ok(())
}

/// # 用投递快照组装一路投递任务
///
/// 字段一一对应三张表，没有额外加工：
///
/// - 内容取 `msg_delivery` 的**已渲染**结果（标题 / 内容 / ext），不再套模板——
///   重投的语义是「把这条消息再发一遍」，模板改了也不该改这次的内容；
/// - `business_trigger_ms` 用原值：它是幂等键的一部分，也是这次投递的业务时刻；
/// - `create_ms` 也用原值：任务里的 `create_ms` 指「这条消息什么时候被创建」，
///   不是「什么时候重投的」；重投时刻看渠道日志的 `receive_ms`。
fn build_task(delivery: &MsgDeliveryVo, item: &RedeliverTask) -> DeliveryMsg {
    DeliveryMsg {
        persisted: true,
        delivery_id: Some(delivery.id.0),
        delivery_target_id: Some(item.delivery_target_id),
        delivery_channel_id: Some(item.delivery_channel_id),
        event_code: delivery.event_code.clone(),
        message_id: delivery.message_id.0,
        event_source_id: delivery.event_source_id.0,
        message_category_id: delivery.message_category_id.0,
        business_id: Some(delivery.business_id.0),
        business_trigger_ms: Some(delivery.business_trigger_ms.0),
        first_trigger_ms: delivery.first_trigger_ms.map(|ms| ms.0),
        first_receive_ms: delivery.first_receive_ms.map(|ms| ms.0),
        title: delivery.title.clone(),
        content: delivery.content.clone(),
        ext: delivery.ext.clone(),
        create_ms: delivery.create_ms.0,
        // 模拟标记随任务带下去：进死信时死信表要按它判性质
        simulated: delivery.simulated,
        // 手工重投：渠道日志的 detail 会因此带上「手工重投」前缀
        redeliver: true,
        target_category_code: item.target_category_code.clone(),
        target_category_id: item.target_category_id,
        target_id: item.target_id.clone(),
        channel_id: item.channel_id,
        channel_code: item.channel_code.clone(),
    }
}

/// 构造一个参数校验错误
fn validation_error(code: &'static str, message: String) -> SvcError {
    SvcError::Validation(ValidationError::new(code).with_message(message.into()))
}
