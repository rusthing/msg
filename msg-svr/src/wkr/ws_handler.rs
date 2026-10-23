//! # 大屏长连接入口
//!
//! 大屏带上 `targetId` 连上来，服务端按 `target_id` 把它登记进
//! [`crate::wkr::conn`]；之后 websocket 渠道的投递任务就按这个 ID 找到连接。
//!
//! ## 握手
//!
//! ```text
//! ws://host:port/msg/ws?targetId=8888&token=xxxx
//! ```
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
use crate::dto::MsgChannelQueryDto;
use crate::svc::MsgChannelSvc;
use crate::wkr::conn;
use axum::extract::Query;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use msg_api::cst::MSG_CHANNEL_CODE_WEBSOCKET;
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
    /// 握手令牌，渠道表未配置时可省略
    pub token: Option<String>,
}

/// 渠道表里的 websocket 配置
#[derive(Debug, Default, Deserialize)]
struct WsChannelOptions {
    #[serde(default)]
    token: String,
}

/// # 升级为 websocket
pub async fn upgrade(ws: WebSocketUpgrade, Query(query): Query<WsQuery>) -> Response {
    match check_token(query.token.as_deref()).await {
        Ok(()) => ws.on_upgrade(move |socket| handle(socket, query.target_id)),
        Err(status) => status.into_response(),
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
async fn handle(socket: WebSocket, target_id: u64) {
    let config = get_msg_config();

    let (sender, mut receiver) = mpsc::channel::<String>(config.ws.send_queue_size.max(1));
    // 连接结束（正常、异常、panic）都会摘掉注册项
    let _guard = conn::register(target_id, sender);

    let mut shutdown = conn::shutdown_receiver();
    let (mut sink, mut stream) = socket.split();

    let mut ticker = tokio::time::interval(Duration::from_secs(
        config.ws.ping_interval_secs.max(1),
    ));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    info!(
        "大屏已连接: target_id={target_id}, 当前连接数={}",
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
        "大屏已断开: target_id={target_id}, 当前连接数={}",
        conn::connection_count()
    );
}
