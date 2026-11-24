//! # 大屏长连接入口
//!
//! 大屏连上来时上报它属于哪个目标类别（`msg_target_category.code`，大屏是
//! `screen`），服务端按这个编码把连接登记进 [`crate::wkr::conn`]；投递任务里
//! 带着同一个编码，websocket 渠道拿它对上就发。
//!
//! 编码由发布方配置（dtw 的 `[dtw.target] category-code`），服务端不预设：
//! 目标类别是消息中心的共享字典，大屏只是其中一项，这里只负责按上报值分桶。
//!
//! ## 握手
//!
//! ```text
//! ws://host:port/msg/ws?targetCategory=screen&token=xxxx
//! ```
//!
//! `targetCategory` 必填：它是这条连接唯一的「我是谁」信息，缺了服务端不知道
//! 该把它归到哪一类，等于连上一个永远收不到消息的壳子，直接 400 拒绝。
//!
//! `token` 的期望值来自这个 `targetCategory` 在 `msg_target_category_channel`
//! 里绑定的 websocket 渠道的 `msg_channel.options`：
//!
//! ```json
//! { "token": "xxxx" }
//! ```
//!
//! 按类别取令牌，是因为类别本身就是「这条连接是谁」：大屏是 `screen` 一种类别，
//! 另一种业务的大屏就是另一种类别，各自绑自己的 websocket 渠道、各配各的令牌，
//! 不需要在握手参数里再传渠道实例。一个类别绑了多条 websocket 渠道时取令牌并集
//! （任一匹配即放行）。
//!
//! 拒绝的情形：类别在 `msg_target_category` 里不存在（400，投递永远命中不到它）；
//! 该类别没绑 websocket 渠道、或绑的全被禁用（503）；`options` 存在但解析不出
//! JSON（503，那是「本来要配令牌但写坏了」，不能退化成不校验）。令牌全为空表示
//! 不校验（联调用）。

use crate::config::get_msg_config;
use crate::dao::{MsgChannelDao, MsgTargetCategoryChannelDao, MsgTargetCategoryDao};
use crate::wkr::conn;
use axum::extract::Query;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use msg_api::cst::ChannelCode;
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
    /// 目标类别编码（`msg_target_category.code`），与投递任务里的同名字段比对
    pub target_category: String,
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
    let category_code = query.target_category.trim().to_string();
    if category_code.is_empty() {
        warn!("握手缺少目标类别编码，拒绝连接");
        return StatusCode::BAD_REQUEST.into_response();
    }

    if let Err(status) = check_token(&category_code, query.token.as_deref()).await {
        return status.into_response();
    }

    ws.on_upgrade(move |socket| handle(socket, category_code))
}

/// 校验握手令牌
///
/// 顺序是「类别 → 该类别绑定的 websocket 渠道 → 令牌」：
///
/// 1. 类别不存在直接拒绝：连接登记进去也只会在投递时被当成没人接收的类别，
///    早点报错比让大屏挂着一个永远空白的连接好；
/// 2. 类别没绑 websocket 渠道、或绑的渠道全被禁用，拒绝——没有渠道配置就没人
///    往这类连接投递；
/// 3. 令牌取这些渠道 `options.token` 的并集：任何一条启用渠道配了非空令牌就
///    按它校验，所有渠道都没配才不校验（联调场景）。`options` 解析不了直接拒绝，
///    那是运营本意要配令牌却写坏了，按「未配置」放行等于把鉴权关掉。
async fn check_token(category_code: &str, token: Option<&str>) -> Result<(), StatusCode> {
    use crate::mo::msg_channel::{Column as ChannelCol, Model as ChannelModel};
    use crate::mo::msg_target_category::{Column as CategoryCol, Model as CategoryModel};
    use crate::mo::msg_target_category_channel::Column as CategoryChannelCol;
    use sea_orm::{ColumnTrait, Condition};

    let db = get_db_conn().map_err(|e| {
        warn!("校验 websocket 令牌失败，取数据库连接出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // 1. 类别必须存在
    let categories: Vec<CategoryModel> = MsgTargetCategoryDao::list_by_condition(
        Condition::all().add(CategoryCol::Code.eq(category_code)),
        &None,
        db.as_ref(),
    )
    .await
    .map_err(|e| {
        warn!("校验 websocket 令牌失败，查询目标类别出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let Some(category) = categories.into_iter().next() else {
        warn!("目标类别不存在，拒绝连接: category={category_code}");
        return Err(StatusCode::BAD_REQUEST);
    };

    // 2. 该类别绑定的 websocket 渠道（启用）
    let binds: Vec<crate::mo::msg_target_category_channel::Model> =
        MsgTargetCategoryChannelDao::list_by_condition(
            Condition::all().add(CategoryChannelCol::TargetCategoryId.eq(category.id)),
            &None,
            db.as_ref(),
        )
        .await
        .map_err(|e| {
            warn!("校验 websocket 令牌失败，查询类别渠道绑定出错: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let channel_ids: Vec<i64> = binds.iter().map(|bind| bind.channel_id).collect();
    if channel_ids.is_empty() {
        warn!("目标类别没有绑定任何渠道，拒绝连接: category={category_code}");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let channels: Vec<ChannelModel> = MsgChannelDao::list_by_condition(
        Condition::all()
            .add(ChannelCol::Id.is_in(channel_ids))
            .add(ChannelCol::ChannelType.eq(ChannelCode::Websocket.as_str()))
            .add(ChannelCol::Enabled.eq(true)),
        &None,
        db.as_ref(),
    )
    .await
    .map_err(|e| {
        warn!("校验 websocket 令牌失败，查询渠道出错: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if channels.is_empty() {
        warn!("目标类别没有启用中的 websocket 渠道，拒绝连接: category={category_code}");
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    // 3. 令牌并集
    let mut expected = Vec::new();
    for channel in &channels {
        let token = match channel.options.as_deref().map(str::trim) {
            None | Some("") => String::new(),
            Some(options) => match serde_json::from_str::<WsChannelOptions>(options) {
                Ok(parsed) => parsed.token,
                Err(e) => {
                    warn!(
                        "websocket 渠道 options 解析失败，拒绝连接（请修正渠道配置）: channel_type={}, err={e}",
                        channel.channel_type
                    );
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
            },
        };
        if !token.is_empty() {
            expected.push(token);
        }
    }

    if expected.is_empty() {
        return Ok(());
    }
    match token {
        Some(token) if expected.iter().any(|value| value == token) => Ok(()),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// 一条大屏连接的完整生命周期
async fn handle(socket: WebSocket, category_code: String) {
    let config = get_msg_config();

    let (sender, mut receiver) = mpsc::channel::<String>(config.ws.send_queue_size.max(1));
    // 连接结束（正常、异常、panic）都会摘掉注册项
    let _guard = conn::register(&category_code, sender);

    let mut shutdown = conn::shutdown_receiver();
    let (mut sink, mut stream) = socket.split();

    let mut ticker =
        tokio::time::interval(Duration::from_secs(config.ws.ping_interval_secs.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);

    info!(
        "大屏已连接: category={category_code}, 当前连接数={}",
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
                    debug!("大屏连接读取出错: category={category_code}, error={e}");
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
        "大屏已断开: category={category_code}, 当前连接数={}",
        conn::connection_count()
    );
}
