//! # MSG API 客户端
//!
//! 为 MSG 消息中心提供声明式 Feign HTTP 客户端，
//! 通过 `#[feign]` 宏自动生成各资源的 CRUD 方法。

pub mod api_client;

pub use api_client::MsgChannelApiClient;
pub use api_client::MsgDeliveryApiClient;
pub use api_client::MsgDeliveryChannelApiClient;
pub use api_client::MsgDeliveryChannelLogApiClient;
pub use api_client::MsgDeliveryTargetApiClient;
pub use api_client::MsgMessageApiClient;
pub use api_client::MsgMessageCategoryApiClient;
pub use api_client::MsgMessageChannelApiClient;
pub use api_client::MsgMessageQueueApiClient;
pub use api_client::MsgMessageSourceApiClient;
pub use api_client::MsgMessageTargetApiClient;
pub use api_client::MsgTargetCategoryApiClient;
pub use msg_api::dto;
pub use msg_api::vo;