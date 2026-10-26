//! # 库表的 CRUD DTO
//!
//! 一个结构对应 `model/create-table-pgsql.sql` 里的一张表，字段与列一一对应：
//!
//! - **业务列**在这里显式声明，顺序与建表脚本一致，注释取自 `comment on column`；
//! - **主键与审计列**（`id` / `creater_id` / `create_ms` / `updater_id` / `update_ms`）
//!   由 `crud_dto` 宏生成，结构体里不再重复声明 —— 所以字段数比列数少 5 个是对的；
//! - 列为 `not null` 且没有库默认值时，字段写非 `Option` 类型，新增时由宏自动加必填校验；
//!   列有库默认值时标 `#[db_default]`，未传就把该列交给库里的默认值。
//!
//! 改表结构时三处要一起改：本 DTO、同名 VO、`mo` 实体。

pub mod msg_channel_dto;
pub mod msg_dead_letter_dto;
pub mod msg_delivery_channel_dto;
pub mod msg_delivery_channel_log_dto;
pub mod msg_delivery_dto;
pub mod msg_delivery_queue_dto;
pub mod msg_delivery_target_dto;
pub mod msg_event_source_dto;
pub mod msg_message_category_dto;
pub mod msg_message_dto;
pub mod msg_message_target_dto;
pub mod msg_publish_queue_dto;
pub mod msg_target_category_channel_dto;
pub mod msg_target_category_dto;

pub use msg_channel_dto::*;
pub use msg_dead_letter_dto::*;
pub use msg_delivery_channel_dto::*;
pub use msg_delivery_channel_log_dto::*;
pub use msg_delivery_dto::*;
pub use msg_delivery_queue_dto::*;
pub use msg_delivery_target_dto::*;
pub use msg_event_source_dto::*;
pub use msg_message_category_dto::*;
pub use msg_message_dto::*;
pub use msg_message_target_dto::*;
pub use msg_publish_queue_dto::*;
pub use msg_target_category_channel_dto::*;
pub use msg_target_category_dto::*;
