//! # MSG 刷新信号主题常量
//!
//! 本地缓存的跨节点同步通过 JetStream 刷新信号完成：
//! 任一节点写库后发布信号，所有节点收到信号后从 DB 全量重建缓存。
//!
//! ## 命名规范
//!
//! ```text
//! refresh.{服务}.{刷新对象}
//! ```
//!
//! 主题由 [`RefreshSubject`] 承载，只能经 `robotech::refresh_subject!` 宏
//! 构造——段数、前缀、字符集在 `const` 上下文校验，写错直接编译失败。
//!
//! - 第一段固定为 `refresh`，标识「刷新信号」这一框架能力。所有服务的刷新
//!   主题都落在这个命名空间下，`nats stream ls` 里聚成一族，一眼可辨；
//!   与业务主题（`msg.message` / `msg.delivery`）完全隔离，不会被业务侧的
//!   通配 Stream 吞掉；
//! - 第二段为服务名，第三段标识「哪一份缓存 / 配置」，新增缓存时只追加
//!   末段，前缀规则不变；
//! - 末段内的多词标识用 `-` 连接（不允许 `_`，会被类型校验拒绝）：
//!   Stream 名由主题把 `.` 换成 `_` 派生，若 token 内用 `_`，
//!   `a.b_c` 与 `a_b.c` 会派生出同一个 Stream 名。
//!
//! | 主题 | 刷新对象 |
//! |------|----------|
//! | `refresh.msg.message` | 消息及其投递配置（模板、渠道、来源、类别、目标） |
//! | `refresh.msg.queue` | 消息队列订阅 |

use robotech::refresh::RefreshSubject;

/// 消息缓存刷新主题
///
/// 消息模板、事件来源、类别、目标类别、渠道等数据变更时发布，
/// 各节点收到后全量重建消息缓存。
pub const MSG_MESSAGE_REFRESH_SUBJECT: RefreshSubject =
    robotech::refresh_subject!("msg", "message");

/// 消息队列刷新主题
///
/// 消息队列变更时发布，各节点收到后直接从 DB 查询队列并同步 NATS 订阅，
/// 不经过缓存。
pub const MSG_QUEUE_REFRESH_SUBJECT: RefreshSubject = robotech::refresh_subject!("msg", "queue");
