//! # 工具模块
//!
//! - [`refresh_utils`] — 消息队列刷新装配，`main.rs` 启动时调用 [`setup_message_queue_refresh`]
//! - [`template_utils`] — 消息模板渲染

mod refresh_utils;
mod template_utils;

pub use refresh_utils::*;
pub(crate) use template_utils::*;

/// 当前服务端毫秒时间戳（UTC）
///
/// 专供审计列（`create_ms` / `update_ms`）使用。业务时间戳要的是「这件事什么时候
/// 发生」，审计列要的是「这一行什么时候真正落库」，两者差着渲染、建任务、写库
/// 的耗时。拿业务时刻回填审计列，会让两列恒等，等于把这段耗时从库里抹掉。
pub(crate) fn server_now_ms() -> u64 {
    chrono::Utc::now().timestamp_millis() as u64
}
