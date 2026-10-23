//! # 大屏长连接入口
//!
//! 大屏带上 `targetId` 与 `targetCategory` 连上来，服务端把类别编码解析成
//! `msg_target_category.id`，再按 (类别, 目标) 登记进 [`crate::wkr::conn`]；
//! 之后 websocket 渠道的投递任务就按这对坐标找到连接——指定目标的直接命中，
//! 按类别广播的遍历该类别下的全部目标。
//!
//! ## 握手
//!
//! ```text
//! ws://host:port/msg/ws?targetId=8888&targetCategory=screen&token=xxxx
//! ```
//!
//! `targetCategory` 是 `msg_target_category.code`，可以省略，默认 `screen`
//! （大屏）——老客户端只报 `targetId` 也能连上。传了就必须能在类别表里查到，
//! 否则拒绝：消息是按类别登记的，连一个不存在的类别上，永远不会有消息发过来，
//! 与其让它安静地挂着，不如握手时就报错。
//!
//! `token` 的期望值来自 `msg_channel.options`（websocket 那条渠道）：
//!
//! ```json
//! { "token": "xxxx" }
//! ```
//!
//! `token` 为空表示不校验（联调用）；渠道表里没有 websocket 这条渠道时
//! 直接拒绝——没有渠道配置就没人会往这个渠道投递，连上也收不到消息。

use crate::config::get_msg_config;
use crate::dto::{MsgChannelQueryDto, MsgTargetCategoryQueryDto};
use crate::svc::{MsgChannelSvc, MsgTargetCategorySvc};
use crate::wkr::conn;
use axum::extract::Query;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use msg_api::cst::{MSG_CHANNEL_CODE_WEBSOCKET, MSG_TARGET_CATEGORY_CODE_SCREEN};
use robotech::db::get_db_conn;
use serde::Deserialize;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tracing::{debug, info, warn};

/// 连接参数
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsQuery {
    /// 目标 ID，与 `msg_delivery_target.target_id` 对应
    pub target_id: u64,
    /// 目标类别编码（`msg_target_category.code`），省略时按大屏处理
    #[serde(default = "default_target_category")]
    pub target_category: String,
    /// 握手令牌，渠道表未配置时可省略
    pub token: Option<String>,
}

fn default_target_category() -> String {
    MSG_TARGET_CATEGORY_CODE_SCREEN.to_string()
}

/// 渠道表里的 websocket 配置
#[derive(Debug, Default, Deserialize)]
struct WsChannelOptions {
    #[serde(default)]
    token: String,
}

/// # 升级为 websocket
pub async fn upgrade(ws: WebSocketUpgrade, Query(query): Query<WsQuery>) -> Response {
    if let Err(status) = check_token(query.token.as_deref()).await {
        return status.into_response();
    }

    match resolve_target_category(&query.target_category).await {
        Ok(category_id) => {
            ws.on_upgrade(move |socket| handle(socket, category_id, query.target_id))
        }
        Err(status) => status.into_response(),
    }
}

/// # 把目标类别编码解析成 ID
///
/// 类别表是消息中心里的共享字典（用户 / 角色 / 分组 / 大屏），大屏连上来时
/// 只报编码，ID 由服务端查，客户端不需要知道库里的主键。
///
/// 查不到就拒绝：消息目标登记的是类别 ID，连一个不存在的类别上，广播永远
/// 匹配不到它，表现为「连上了但收不到消息」，比握手失败更难排查。
async fn resolve_target_category(code: &str) -> Result<u64, StatusCode> {
    let db = get_db_conn().map_err(|e| {
        warn!("解析目标类别失败，取数据库连接出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ro = MsgTargetCategorySvc::get_by_query_dto(
        MsgTargetCategoryQueryDto {
            code: Some(code.to_string()),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await
    .map_err(|e| {
        warn!("解析目标类别失败，查询出错: code={code}, error={e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match ro.extra {
        Some(category) => Ok(category.id.0),
        None => {
            warn!("目标类别不存在，拒绝连接: code={code}");
            Err(StatusCode::SERVICE_UNAVAILABLE)
        }
    }
}

/// 校验握手令牌
async fn check_token(token: Option<&str>) -> Result<(), StatusCode> {
    let db = get_db_conn().map_err(|e| {
        warn!("校验 websocket 令牌失败，取数据库连接出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let ro = MsgChannelSvc::get_by_query_dto(
        MsgChannelQueryDto {
            code: Some(MSG_CHANNEL_CODE_WEBSOCKET.to_string()),
            ..Default::default()
        },
        Some(db.as_ref()),
    )
    .await
    .map_err(|e| {
        warn!("校验 websocket 令牌失败，查询渠道出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let Some(channel) = ro.extra else {
        warn!("未配置 websocket 渠道（msg_channel.code={MSG_CHANNEL_CODE_WEBSOCKET}），拒绝连接");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    };

    let expected = channel
        .options
        .as_deref()
        .and_then(|options| match serde_json::from_str::<WsChannelOptions>(options) {
            Ok(options) => Some(options.token),
            Err(e) => {
                warn!("websocket 渠道 options 解析失败，按未配置令牌处理: {e}");
                None
            }
        })
        .unwrap_or_default();

    // 没配令牌就是不校验，方便联调
    if expected.is_empty() {
        return Ok(());
    }
    if token == Some(expected.as_str()) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// 一条大屏连接的完整生命周期
async fn handle(socket: WebSocket, category_id: u64, target_id: u64) {
    let config = get_msg_config();

    let (sender, mut receiver) = mpsc::channel::<String>(config.ws.send_queue_size.max(1));
    // 连接结束（正常、异常、panic）都会摘掉注册项
    let _guard = conn::register(category_id, target_id, sender);

    let mut shutdown = conn::shutdown_receiver();
    let (mut sink, mut stream) = socket.split();

    let mut ticker = tokio::time::interval(Duration::from_secs(
        config.ws.ping_interval_secs.max(1),
    ));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    info!(
        "大屏已连接: category_id={category_id}, target_id={target_id}, 当前连接数={}",
        conn::connection_count()
    );

    loop {
        tokio::select! {
            incoming = stream.next() => match incoming {
                // 客户端关闭或连接断开
                None | Some(Ok(Message::Close(_))) => break,
                // 大屏目前只收不发，收到什么内容都忽略
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    debug!("大屏连接读取出错: target_id={target_id}, error={e}");
                    break;
                }
            },
            outgoing = receiver.recv() => match outgoing {
                Some(text) => {
                    if sink.send(Message::Text(text.into())).await.is_err() {
                        break;
                    }
                }
                // 注册表里的发送端已经全部释放
                None => break,
            },
            _ = ticker.tick() => {
                // 写失败说明连接已断；写完就关，避免僵尸连接一直占着注册表
                if sink.send(Message::Ping(Bytes::new())).await.is_err() {
                    break;
                }
            },
            _ = shutdown.recv() => {
                // 服务要重启/重建订阅：主动发 Close，否则框架的优雅关停会一直等
                let _ = sink.send(Message::Close(None)).await;
                break;
            }
        }
    }

    info!(
        "大屏已断开: category_id={category_id}, target_id={target_id}, 当前连接数={}",
        conn::connection_count()
    );
}
