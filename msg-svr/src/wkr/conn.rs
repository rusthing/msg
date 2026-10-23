//! # 大屏连接注册表
//!
//! 维护「目标类别 → 目标 ID → 当前连着的连接」的映射，websocket 渠道发送时
//! 按目标定位连接，把消息塞进它的待发队列。
//!
//! ## 为什么要带目标类别
//!
//! 目标 ID 只在类别内唯一：大屏 8888 与用户 8888 是两个目标。websocket 渠道
//! 目前只服务大屏，但注册表按 (类别, 目标) 记一份，发送侧就不用假设「这个渠道
//! 只可能有一种目标」，以后同一条渠道服务多类目标也不用改结构。
//!
//! 两级 map 而不是一级 `(类别, 目标)` 元组：投递任务有两种粒度——
//!
//! - **指定目标**：按 (类别, 目标) 直接定位；
//! - **按类别广播**（`target_id` 为空）：只给类别，遍历该类别下的全部目标。
//!
//! 广播是定位消息这类高频消息的常态，一级 map 每次都要扫全部连接，
//! 屏一多就白跑。
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

/// 注册表：`目标类别 ID` → `目标 ID` → 该目标的连接列表
static CONNS: OnceLock<RwLock<HashMap<u64, HashMap<u64, Vec<Conn>>>>> = OnceLock::new();

/// 连接序号分配器
static NEXT_CONN_ID: AtomicU64 = AtomicU64::new(1);

/// 关闭信号发送端
static SHUTDOWN: OnceLock<broadcast::Sender<()>> = OnceLock::new();

fn conns() -> &'static RwLock<HashMap<u64, HashMap<u64, Vec<Conn>>>> {
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
    category_id: u64,
    target_id: u64,
    conn_id: u64,
}

impl Drop for ConnGuard {
    fn drop(&mut self) {
        let mut guard = conns().write().unwrap();
        let Some(targets) = guard.get_mut(&self.category_id) else {
            return;
        };
        if let Some(list) = targets.get_mut(&self.target_id) {
            list.retain(|conn| conn.id != self.conn_id);
            if list.is_empty() {
                targets.remove(&self.target_id);
            }
        }
        // 该类别下一条连接都不剩时，把类别这层也摘掉，别留空壳
        if targets.is_empty() {
            guard.remove(&self.category_id);
        }
    }
}

/// # 注册一条连接
///
/// 同一个 `target_id` 可以有多条连接（大屏刷新页面、多开），都会收到消息。
///
/// `category_id` 是 `msg_target_category.id`，由握手时上报的类别编码解析而来。
pub fn register(category_id: u64, target_id: u64, sender: mpsc::Sender<String>) -> ConnGuard {
    let conn_id = NEXT_CONN_ID.fetch_add(1, Ordering::Relaxed);
    conns()
        .write()
        .unwrap()
        .entry(category_id)
        .or_default()
        .entry(target_id)
        .or_default()
        .push(Conn {
            id: conn_id,
            sender,
        });
    ConnGuard {
        category_id,
        target_id,
        conn_id,
    }
}

/// # 向目标推送
///
/// `target_id` 为 `Some` 时只发给该目标；为 `None` 时发给该类别下的全部目标
/// （按类别广播）。两种粒度都由这里落地，调用方不用自己遍历连接。
///
/// ## 返回值
/// `true` 表示至少写入了一条连接的队列；`false` 表示本实例没有可投的连接，
/// 或所有连接的队列都满了/已关闭。
///
/// 广播时返回 `false` 不代表投递失败：屏可能连在别的实例上，那边会发。
pub fn send(category_id: u64, target_id: Option<u64>, text: String) -> bool {
    let guard = conns().read().unwrap();
    let Some(targets) = guard.get(&category_id) else {
        return false;
    };

    match target_id {
        Some(target_id) => targets
            .get(&target_id)
            .map(|list| push(list, target_id, &text))
            .unwrap_or(false),
        None => {
            let mut delivered = false;
            for (target_id, list) in targets {
                delivered |= push(list, *target_id, &text);
            }
            delivered
        }
    }
}

/// 把一份内容塞进一组连接的待发队列
fn push(list: &[Conn], target_id: u64, text: &str) -> bool {
    let mut delivered = false;
    for conn in list {
        match conn.sender.try_send(text.to_string()) {
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
    conns()
        .read()
        .unwrap()
        .values()
        .map(|targets| targets.values().map(Vec::len).sum::<usize>())
        .sum()
}
