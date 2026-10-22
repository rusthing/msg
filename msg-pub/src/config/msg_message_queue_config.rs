//! # 消息队列配置
//!
//! 业务方在自身配置里声明自己发布消息用的队列（NATS subject），启动时转成
//! AddDto 直接调用 msg-svr 的 `add` 接口注册。与
//! [`MsgEventSourceConfig`](crate::MsgEventSourceConfig) 一样只做声明，
//! 不封装注册流程。
//!
//! `msg_message_queue` 的用途是让消息中心知道「要订阅哪些队列」：客户端把
//! 自己发消息的 subject 注册上来，消息中心热更新订阅它。因此这里的 `code`
//! 必须与发布时用的 subject 一致——两边写岔就是「注册的队列没人发，发的
//! subject 没人订阅」。

use derive_setters::Setters;
use o2o::o2o;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

#[derive(o2o, Debug, Clone, Serialize, Deserialize, Setters, TypedBuilder)]
#[owned_into(msg_api_client::dto::MsgMessageQueueAddDto)]
#[ghosts(
    id: Default::default(),
    _current_ms: Default::default(),
    _current_user_id: robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID.into(),
)]
#[serde(rename_all = "kebab-case")]
pub struct MsgMessageQueueConfig {
    /// 队列编码，同时是发布消息用的 NATS subject
    #[into(Some(~))]
    pub code: String,
    /// 名称（唯一）
    #[into(Some(~))]
    pub name: String,
    /// 是否持久化：`true` 走 JetStream（服务端订阅会创建覆盖该 subject 的
    /// stream），`false` 走 Core NATS
    #[into(Some(~))]
    #[serde(default = "persisted_default")]
    #[builder(default = persisted_default())]
    pub persisted: bool,
    /// 备注
    #[into(Some(~))]
    #[serde(default)]
    #[builder(default)]
    pub remark: Option<String>,
}

fn persisted_default() -> bool {
    true
}
