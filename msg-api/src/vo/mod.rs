//! # 库表的查询 VO
//!
//! 与 `dto` 同名同表，区别是**字段为该表全部列**：VO 不经过宏注入，主键与审计列
//! （`id` / `creater_id` / `create_ms` / `updater_id` / `update_ms`）也必须显式写出来。
//! 顺序与 `model/create-table-pgsql.sql` 一致，注释取自 `comment on column`。

mod msg_channel_vo;
mod msg_dead_letter_vo;
mod msg_delivery_channel_log_vo;
mod msg_delivery_channel_vo;
mod msg_delivery_target_vo;
mod msg_delivery_vo;
mod msg_event_source_vo;
mod msg_message_category_vo;
mod msg_message_queue_vo;
mod msg_message_target_vo;
mod msg_message_vo;
mod msg_target_category_channel_vo;
mod msg_target_category_vo;

pub use msg_channel_vo::*;
pub use msg_dead_letter_vo::*;
pub use msg_delivery_channel_log_vo::*;
pub use msg_delivery_channel_vo::*;
pub use msg_delivery_target_vo::*;
pub use msg_delivery_vo::*;
pub use msg_event_source_vo::*;
pub use msg_message_category_vo::*;
pub use msg_message_queue_vo::*;
pub use msg_message_target_vo::*;
pub use msg_message_vo::*;
pub use msg_target_category_channel_vo::*;
pub use msg_target_category_vo::*;