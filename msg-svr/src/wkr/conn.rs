//! # 大屏连接注册表
//!
//! 维护「目标 ID → 当前连着的连接」的映射，websocket 渠道发送时按
//! `target_id` 找到连接把消息塞进它的待发队列。
//!
//! ## 为什么 key 只有 target_id
//!
//! websocket 渠道只服务大屏这一类目标（`msg_target_category` 里的「大屏」），
//! 一类目标一个渠道，不会出现「用户 8888」与「大屏 8888」同时挂在 websocket 上，
//! 所以不需要再带类别前缀。
//!
//! ## 队列与背压
//!
//! 每条连接一个 `mpsc`，发送方 `try_send` 不阻塞投递线程：
//!
//! - 队列满（大屏消费不过来）→ 丢弃这条消息并打 warn，绝不等待；
//! - 连接已关闭 → 忽略，连接自己的任务结束时会把注册项摘掉。
//!
//! 大屏要的是「最新状态」，不需要保证一条不漏，所以这里不做重试。
//!
//! ## 关闭信号
//!
//! WS 长连接永远不会自己结束，而框架的优雅关停会等所有连接完成。
//! 因此这里维护一个 `broadcast`：`setup_wkr` 重建订阅前先发信号，
//! 每条连接收到后发 Close 帧退出，否则进程会卡在关停上。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};
use tokio::sync::{broadcast, mpsc};
use tracing::warn;

/// 单条连接
#[derive(Debug)]
struct Conn {
    /// 连接序号，摘除时按它定位
    id: u64,
    /// 待发队列
    sender: mpsc::Sender<String>,
}

/// 注册表：`target_id` → 该目标的连接列表
static CONNS: OnceLock<RwLock<HashMap<u64, Vec<Conn>>>> = OnceLock::new();

/// 连接序号分配器
static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// 关闭信号发送端
static SHUTDOWN: OnceLock<broadcast::Sender<()>> = OnceLock::new();

fn conns() -> &'static RwLock<HashMap<u64, Vec<Conn>>> {
    CONNS.get_or_init(|| RwLock::new(HashMap::new()))
}

fn shutdown_sender() -> &'static broadcast::Sender<()> {
    SHUTDOWN.get_or_init(|| broadcast::channel(1).0)
}

/// # 订阅关闭信号
///
/// 连接任务在 `select!` 里等这个信号，收到就发 Close 帧退出。
pub fn shutdown_receiver() -> broadcast::Receiver<()> {
    shutdown_sender().subscribe()
}

/// # 广播关闭信号
///
/// 用于配置热更新重建订阅、进程退出前主动断开所有长连接。
pub fn close_all() {
    // 没有订阅者时返回 Err，属于正常情况
    let _ = shutdown_sender().send(());
}

/// # 连接注册凭据
///
/// 持有期间连接有效；`Drop` 时自动从注册表摘除，
/// 连接任务无论正常退出还是 panic 都不会留下僵尸连接。
#[derive(Debug)]
pub struct ConnGuard {
    target_id: u64,
    conn_id: u64,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut guard = conns().write().unwrap();
        if let Some(list) = guard.get_mut(&self.target_id) {
            list.retain(|conn| conn.id != self.conn_id);
            if list.is_empty() {
                guard.remove(&self.target_id);
            }
        }
    }
}

/// # 注册一条连接
///
/// 同一个 `target_id` 可以有多条连接（大屏刷新页面、多开），都会收到消息。
pub fn register(target_id: u64, sender: mpsc::Sender<String>) -> ConnGuard {
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
    conns()
        .write()
        .unwrap()
        .entry(target_id)
        .or_default()
        .push(Conn {
            id: conn_id,
            sender,
        });
    ConnGuard { target_id, conn_id }
}

/// # 向某个目标的所有连接推送
///
/// ## 返回值
/// `true` 表示至少写入了一条连接的队列；`false` 表示该目标当前没有连接，
/// 或所有连接的队列都满了/已关闭。
pub fn send(target_id: u64, text: String) -> bool {
    let guard = conns().read().unwrap();
    let Some(list) = guard.get(&target_id) else {
        return false;
    };

    let mut delivered = false;
    for conn in list {
        match conn.sender.try_send(text.clone()) {
            Ok(()) => delivered = true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(
                    "连接队列已满，丢弃消息: target_id={}, conn_id={}",
                    target_id, conn.id
                );
            }
            // 连接任务正在退出，注册项马上会被 ConnGuard 摘除
            Err(mpsc::error::TrySendError::Closed(_)) => {}
        }
    }
    delivered
}

/// # 当前连接总数（含同一目标的多条连接）
pub fn connection_count() -> usize {
    conns().read().unwrap().values().map(Vec::len).sum()
}
