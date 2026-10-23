//! # 大屏连接注册表
//!
//! 维护「目标类别编码 → 当前连着的连接」的映射，websocket 渠道发送时按投递
//! 任务里的目标类别编码找到该类别下的全部连接，把消息塞进它们的待发队列。
//!
//! ## 为什么按类别编码、不按目标 ID
//!
//! 大屏是动态接入的：哪块屏连上来才存在，标识还是它自己报的。消息侧登记的是
//! 「发给大屏这一类」（`msg_message_target.target_id` 为空串），投递时也就
//! 只能按类别找连接——屏连上来时上报的是它属于哪一类，不是它要收哪条消息。
//!
//! 类别编码（`msg_target_category.code`）是字符串，连接和投递任务里带的都是它，
//! 不用再回库把编码换成 ID：握手时少一次查询，投递时也不用猜类别表有没有变。
//!
//! 以后真需要按具体目标路由（比如给某个用户单独推），再在这层加一维
//! `类别 → 目标 → 连接`，发送侧按任务里的 `target_id` 决定走哪条路。
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

/// 注册表：`目标类别编码` → 该类别下的连接列表
static CONNS: OnceLock<RwLock<HashMap<String, Vec<Conn>>>> = OnceLock::new();

/// 连接序号分配器
static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// 关闭信号发送端
static SHUTDOWN: OnceLock<broadcast::Sender<()>> = OnceLock::new();

fn conns() -> &'static RwLock<HashMap<String, Vec<Conn>>> {
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
    category_code: String,
    conn_id: u64,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut guard = conns().write().unwrap();
        let Some(list) = guard.get_mut(&self.category_code) else {
            return;
        };
        list.retain(|conn| conn.id != self.conn_id);
        // 该类别下一条连接都不剩时，把类别这层也摘掉，别留空壳
        if list.is_empty() {
            guard.remove(&self.category_code);
        }
    }
}

/// # 注册一条连接
///
/// 同一个类别可以有多条连接（多块屏、大屏刷新页面、多开），都会收到消息。
///
/// `category_code` 是握手时上报的目标类别编码（`msg_target_category.code`）。
pub fn register(category_code: &str, sender: mpsc::Sender<String>) -> ConnGuard {
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
    conns()
        .write()
        .unwrap()
        .entry(category_code.to_string())
        .or_default()
        .push(Conn {
            id: conn_id,
            sender,
        });
    ConnGuard {
        category_code: category_code.to_string(),
        conn_id,
    }
}

/// # 向某个目标类别下的全部连接推送
///
/// ## 返回值
/// `true` 表示至少写入了一条连接的队列；`false` 表示本实例该类别下没有可投的
/// 连接，或所有连接的队列都满了/已关闭。
///
/// 返回 `false` 不代表投递失败：屏可能连在别的实例上，那边会发。
pub fn send(category_code: &str, text: String) -> bool {
    let guard = conns().read().unwrap();
    let Some(list) = guard.get(category_code) else {
        return false;
    };
    push(list, category_code, &text)
}

/// 把一份内容塞进一组连接的待发队列
fn push(list: &[Conn], category_code: &str, text: &str) -> bool {
    let mut delivered = false;
    for conn in list {
        match conn.sender.try_send(text.to_string()) {
            Ok(()) => delivered = true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                warn!(
                    "连接队列已满，丢弃消息: category={}, conn_id={}",
                    category_code, conn.id
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
    conns()
        .read()
        .unwrap()
        .values()
        .map(Vec::len)
        .sum()
}
