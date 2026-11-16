//! # 投递结果回写
//!
//! 持久化消息（`persisted = true`）发送完成后按三张投递表回写状态：
//!
//! ```text
//! 渠道（msg_delivery_channel）  ← 本次发送结果 + 地址
//!   → 目标（msg_delivery_target）   ← 该目标的渠道全部终态后汇总
//!     → 投递（msg_delivery）        ← 该投递的所有目标全部终态后汇总
//! ```
//!
//! 每次回写都从库里重新统计，而不是在内存里累加：消息可重投、多实例可能
//! 同时回写，只有「按库现状重算」才是幂等的。
//!
//! 非持久化消息不会走到这里——它们没有投递记录（三张表的行 ID 全为空），
//! 也不允许回写。

use crate::dto::{
    MsgDeliveryChannelLogAddDto, MsgDeliveryChannelModifyDto, MsgDeliveryChannelQueryDto,
    MsgDeliveryModifyDto, MsgDeliveryTargetModifyDto, MsgDeliveryTargetQueryDto,
};
use crate::svc::{
    MsgDeliveryChannelLogSvc, MsgDeliveryChannelSvc, MsgDeliverySvc, MsgDeliveryTargetSvc,
};
use crate::utils::server_now_ms;
use crate::wkr::channel::SendOutcome;
use idworker::next_id;
use msg_api::cst::TARGET_ID_ALL;
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::msg::DeliveryMsg;
use robotech::api::U64;
use robotech::db::get_db_conn;
use sea_orm::DatabaseConnection;
use std::time::Duration;
use tracing::{debug, error, warn};

/// 状态回写的尝试次数：库抖一下不该让成功发送的结果永久丢失
const WRITE_ATTEMPTS: usize = 3;

/// 第 n 次失败后的退避（100ms 起，逐次翻倍），总耗时控制在 1s 内
const WRITE_RETRY_BASE_MS: u64 = 100;

/// # 记录一次发送结果
///
/// 只处理持久化消息；非持久化消息直接返回（它们本来就没有投递记录）。
///
/// `receive_ms` 是消费端收到这条投递任务的时刻，由调用方在发送前取：它和
/// `finish_ms`（本次写库时刻）之差是渠道内的发送耗时。
///
/// 返回 `Err` 表示状态最终没写进去。调用方据此决定是否 NAK：**已送达**的
/// 结果不能重投（会让大屏看到重复消息），只告警；**发送失败**的结果可以 NAK
/// 交给 JetStream 重投。
pub async fn record(
    task: &DeliveryMsg,
    outcome: &SendOutcome,
    receive_ms: u64,
) -> Result<(), String> {
    // 手工重投的备注由任务自己带着（`DeliveryMsg.redeliver`）：消费者与
    // 「发布就失败」那条路径都调用本函数，两处都要能标出来，所以判定放在这里
    let note = task.redeliver.then_some(MANUAL_REDELIVER_NOTE);
    record_with_note(task, outcome, receive_ms, note).await
}

/// 手工重投在渠道日志里的备注前缀
///
/// 见 [`record_with_note`]：重投沿用原始 `business_trigger_ms`，没有这个标记，
/// 渠道日志里「端到端耗时两小时」会被当成链路卡顿。
pub const MANUAL_REDELIVER_NOTE: &str = "手工重投";

/// # 记录一次发送结果（带备注）
///
/// `note` 会拼在渠道日志的 `detail` 前面，用来区分「第一次投递」与
/// 「用户手工重投」这两种同样会写进 `msg_delivery_channel_log` 的尝试。
/// 不区分的话，一个隔了两小时的手工重投在日志里看起来就是一次超长延迟的
/// 首发（`business_trigger_ms` 是原始业务时刻，端到端耗时自然很大），
/// 排查的人会去追根本不存在的链路卡顿。
pub async fn record_with_note(
    task: &DeliveryMsg,
    outcome: &SendOutcome,
    receive_ms: u64,
    note: Option<&str>,
) -> Result<(), String> {
    // 广播任务的「本实例没连接」不算失败：屏可能连在别的实例上，那边会把
    // 这条渠道记成送达。这里跟着记一笔失败，就会把别人的成功覆盖掉——
    // 多实例部署下渠道状态完全取决于谁最后回写。所以广播只回写送达/发送失败，
    // 没连接就当这次没轮到本实例。
    //
    // 代价：所有实例都没连接时，渠道状态停在「投递中」。大屏离线本来就不补投，
    // 这个状态没有下游依赖，接受。
    if task.target_id == TARGET_ID_ALL && matches!(outcome, SendOutcome::NoConnection) {
        debug!(
            "广播投递本实例无连接，跳过回写: category={}, event_code={}",
            task.target_category_code, task.event_code
        );
        return Ok(());
    }

    let (Some(delivery_channel_id), Some(delivery_target_id), Some(delivery_id)) = (
        task.delivery_channel_id,
        task.delivery_target_id,
        task.delivery_id,
    ) else {
        if task.persisted {
            warn!(
                "持久化投递任务缺少投递 ID，跳过回写: event_code={}, target_id={}",
                task.event_code, task.target_id
            );
        }
        return Ok(());
    };

    let db = match get_db_conn() {
        Ok(db) => db,
        Err(e) => return Err(format!("取数据库连接失败: {e}")),
    };

    let status = if outcome.is_delivered() {
        DeliverChannelStatus::Success
    } else {
        DeliverChannelStatus::Failed
    };
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;

    // 持久化任务一定带业务触发时间（`process_message` 在幂等那步就拒收了缺
    // 幂等键的消息），真缺了就用接收时间顶上——日志写不进去比时间戳略有偏差更糟
    let business_trigger_ms = task.business_trigger_ms.unwrap_or(receive_ms);

    // ── 1. 渠道日志：一次发送尝试一条，留着排查 ──

    match next_id() {
        Ok(log_id) => {
            let log = MsgDeliveryChannelLogAddDto {
                id: Some(U64(log_id)),
                delivery_channel_id: Some(U64(delivery_channel_id)),
                // 事件起点（业务时钟）：发布方没给就留空，不拿通知时间冒充
                first_trigger_ms: Some(task.first_trigger_ms.map(U64)),
                // 链路第一跳收到事件的时刻（服务端时钟），端到端延迟只信它
                first_receive_ms: Some(task.first_receive_ms.map(U64)),
                receive_ms: Some(U64(receive_ms)),
                finish_ms: Some(Some(U64(now_ms))),
                business_trigger_ms: Some(U64(business_trigger_ms)),
                deliver_channel_status: Some(status),
                address: Some(outcome.address()),
                detail: Some(Some(match note {
                    Some(note) => format!("{note}；{}", outcome.detail()),
                    None => outcome.detail().to_string(),
                })),
                // 审计列写真实落库时刻：`finish_ms` 已经是「发送结束」的业务
                // 时刻，再拿它回填 `create_ms` 只会让两列恒等
                _current_ms: Some(U64(server_now_ms())),
                _current_user_id: U64(0),
            };
            if let Err(e) = MsgDeliveryChannelLogSvc::add(log, Some(db.as_ref())).await {
                error!("写入投递渠道日志失败: {e}");
            }
        }
        Err(e) => error!("生成投递渠道日志ID失败: {e}"),
    }

    // ── 2+3. 渠道状态与逐级汇总：库抖一下先原地重试，别急着把状态丢掉 ──
    //
    // 注意只有这两步重试，渠道日志不重试：日志是「一次发送尝试一条」，重试会
    // 写出重复的尝试记录，排查时反而误导。
    let mut last_error = String::new();
    for attempt in 1..=WRITE_ATTEMPTS {
        let modify = MsgDeliveryChannelModifyDto {
            id: Some(U64(delivery_channel_id)),
            deliver_channel_status: Some(status),
            address: Some(outcome.address()),
            _current_user_id: U64(0),
            ..Default::default()
        };

        let result = async {
            MsgDeliveryChannelSvc::modify(modify, Some(db.as_ref()))
                .await
                .map_err(|e| format!("回写投递渠道状态失败: {e}"))?;
            refresh_target(db.as_ref(), delivery_target_id, delivery_id).await?;
            Ok::<(), String>(())
        }
        .await;

        match result {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_error = e;
                if attempt < WRITE_ATTEMPTS {
                    let delay_ms = WRITE_RETRY_BASE_MS * attempt as u64;
                    warn!(
                        "投递状态回写失败（第 {attempt}/{WRITE_ATTEMPTS} 次），{delay_ms}ms 后重试: {last_error}"
                    );
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                }
            }
        }
    }

    Err(last_error)
}

/// 该目标的渠道全部到终态后，汇总目标状态
async fn refresh_target(
    db: &DatabaseConnection,
    delivery_target_id: u64,
    delivery_id: u64,
) -> Result<(), String> {
    let query = MsgDeliveryChannelQueryDto {
        deliver_target_id: Some(U64(delivery_target_id)),
        ..Default::default()
    };
    let channels = MsgDeliveryChannelSvc::list_by_query_dto(query, Some(db))
        .await
        .map_err(|e| format!("汇总投递目标状态失败，查询渠道出错: {e}"))?
        .extra
        .unwrap_or_default();

    // 还有渠道在投递中（同一目标可能挂了多个渠道），等最后一个回来再汇总
    if channels.is_empty()
        || channels
            .iter()
            .any(|channel| channel.deliver_channel_status == DeliverChannelStatus::Delivering)
    {
        return Ok(());
    }

    // 目标只有一个状态位：任一渠道成功即视为目标可达
    let status = if channels
        .iter()
        .any(|channel| channel.deliver_channel_status == DeliverChannelStatus::Success)
    {
        DeliverTargetStatus::Success
    } else {
        DeliverTargetStatus::Failed
    };

    let modify = MsgDeliveryTargetModifyDto {
        id: Some(U64(delivery_target_id)),
        deliver_target_status: Some(status),
        _current_user_id: U64(0),
        ..Default::default()
    };
    MsgDeliveryTargetSvc::modify(modify, Some(db))
        .await
        .map_err(|e| format!("回写投递目标状态失败: {e}"))?;

    refresh_delivery(db, delivery_id).await
}

/// 该投递的所有目标全部到终态后，汇总投递状态
async fn refresh_delivery(db: &DatabaseConnection, delivery_id: u64) -> Result<(), String> {
    let query = MsgDeliveryTargetQueryDto {
        delivery_id: Some(U64(delivery_id)),
        ..Default::default()
    };
    let targets = MsgDeliveryTargetSvc::list_by_query_dto(query, Some(db))
        .await
        .map_err(|e| format!("汇总投递状态失败，查询目标出错: {e}"))?
        .extra
        .unwrap_or_default();

    if targets.is_empty()
        || targets
            .iter()
            .any(|target| target.deliver_target_status == DeliverTargetStatus::Delivering)
    {
        return Ok(());
    }

    let total = targets.len();
    let success = targets
        .iter()
        .filter(|target| {
            matches!(
                target.deliver_target_status,
                // 已读也是送达之后的状态
                DeliverTargetStatus::Success | DeliverTargetStatus::Read
            )
        })
        .count();

    let status = if success == total {
        DeliverStatus::AllSuccess
    } else if success == 0 {
        DeliverStatus::AllFailed
    } else {
        DeliverStatus::PartialSuccess
    };

    let modify = MsgDeliveryModifyDto {
        id: Some(U64(delivery_id)),
        deliver_status: Some(status),
        _current_user_id: U64(0),
        ..Default::default()
    };
    MsgDeliverySvc::modify(modify, Some(db))
        .await
        .map_err(|e| format!("回写投递状态失败: {e}"))?;
    Ok(())
}
