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
use crate::wkr::channel::SendOutcome;
use idworker::next_id;
use msg_api::cst::TARGET_ID_ALL;
use msg_api::dic::{DeliverChannelStatus, DeliverStatus, DeliverTargetStatus};
use msg_api::msg::DeliveryMsg;
use robotech::api::U64;
use robotech::db::get_db_conn;
use sea_orm::DatabaseConnection;
use tracing::{debug, error, warn};

/// # 记录一次发送结果
///
/// 只处理持久化消息；非持久化消息直接返回（它们本来就没有投递记录）。
pub async fn record(task: &DeliveryMsg, outcome: &SendOutcome) {
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
        return;
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
        return;
    };

    let db = match get_db_conn() {
        Ok(db) => db,
        Err(e) => {
            error!("回写投递结果失败，取数据库连接出错: {e}");
            return;
        }
    };

    let status = if outcome.is_delivered() {
        DeliverChannelStatus::Success
    } else {
        DeliverChannelStatus::Failed
    };
    let now_ms = chrono::Utc::now().timestamp_millis() as u64;

    // ── 1. 渠道日志：一次发送尝试一条，留着排查 ──

    match next_id() {
        Ok(log_id) => {
            let log = MsgDeliveryChannelLogAddDto {
                id: Some(U64(log_id)),
                delivery_channel_id: Some(U64(delivery_channel_id)),
                deliver_channel_status: Some(status),
                address: Some(outcome.address()),
                detail: Some(Some(outcome.detail().to_string())),
                _current_ms: Some(U64(now_ms)),
                _current_user_id: U64(0),
            };
            if let Err(e) = MsgDeliveryChannelLogSvc::add(log, Some(db.as_ref())).await {
                error!("写入投递渠道日志失败: {e}");
            }
        }
        Err(e) => error!("生成投递渠道日志ID失败: {e}"),
    }

    // ── 2. 渠道状态 ──

    let modify = MsgDeliveryChannelModifyDto {
        id: Some(U64(delivery_channel_id)),
        deliver_channel_status: Some(status),
        address: Some(outcome.address()),
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
        ..Default::default()
    };
    if let Err(e) = MsgDeliveryChannelSvc::modify(modify, Some(db.as_ref())).await {
        error!("回写投递渠道状态失败: {e}");
        return;
    }

    // ── 3. 逐级汇总 ──

    refresh_target(db.as_ref(), delivery_target_id, delivery_id).await;
}

/// 该目标的渠道全部到终态后，汇总目标状态
async fn refresh_target(db: &DatabaseConnection, delivery_target_id: u64, delivery_id: u64) {
    let query = MsgDeliveryChannelQueryDto {
        deliver_target_id: Some(U64(delivery_target_id)),
        ..Default::default()
    };
    let channels = match MsgDeliveryChannelSvc::list_by_query_dto(query, Some(db)).await {
        Ok(ro) => ro.extra.unwrap_or_default(),
        Err(e) => {
            error!("汇总投递目标状态失败，查询渠道出错: {e}");
            return;
        }
    };

    // 还有渠道在投递中（同一目标可能挂了多个渠道），等最后一个回来再汇总
    if channels.is_empty()
        || channels
            .iter()
            .any(|channel| channel.deliver_channel_status == DeliverChannelStatus::Delivering)
    {
        return;
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

    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let modify = MsgDeliveryTargetModifyDto {
        id: Some(U64(delivery_target_id)),
        deliver_target_status: Some(status),
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
        ..Default::default()
    };
    if let Err(e) = MsgDeliveryTargetSvc::modify(modify, Some(db)).await {
        error!("回写投递目标状态失败: {e}");
        return;
    }

    refresh_delivery(db, delivery_id).await;
}

/// 该投递的所有目标全部到终态后，汇总投递状态
async fn refresh_delivery(db: &DatabaseConnection, delivery_id: u64) {
    let query = MsgDeliveryTargetQueryDto {
        delivery_id: Some(U64(delivery_id)),
        ..Default::default()
    };
    let targets = match MsgDeliveryTargetSvc::list_by_query_dto(query, Some(db)).await {
        Ok(ro) => ro.extra.unwrap_or_default(),
        Err(e) => {
            error!("汇总投递状态失败，查询目标出错: {e}");
            return;
        }
    };

    if targets.is_empty()
        || targets
            .iter()
            .any(|target| target.deliver_target_status == DeliverTargetStatus::Delivering)
    {
        return;
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

    let now_ms = chrono::Utc::now().timestamp_millis() as u64;
    let modify = MsgDeliveryModifyDto {
        id: Some(U64(delivery_id)),
        deliver_status: Some(status),
        _current_ms: Some(U64(now_ms)),
        _current_user_id: U64(0),
        ..Default::default()
    };
    if let Err(e) = MsgDeliverySvc::modify(modify, Some(db)).await {
        error!("回写投递状态失败: {e}");
    }
}
