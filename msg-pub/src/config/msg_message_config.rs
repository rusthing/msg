//! # 消息模板配置
//!
//! 业务方在自身配置里声明要发布的消息（模板、所属类别、所属事件来源），
//! 启动时转成 AddDto 直接调用 msg-svr 的 `add` 接口注册。
//! 与 [`MsgEventSourceConfig`](crate::MsgEventSourceConfig) 一样只做声明，
//! 不封装注册流程——是否需要判重、何时注册由业务方自己决定。

use derive_setters::Setters;
use msg_api_client::dto::MsgMessageAddDto;
use o2o::o2o;
use robotech::api::U64;
use robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID;
use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

/// # 消息类别配置
///
/// 对应 `msg_message_category`。类别是 msg 侧的消息分类（缓存组装时
/// **必须有类别**，无类别的消息不会进模板缓存）。
#[derive(o2o, Debug, Clone, Serialize, Deserialize, Setters, TypedBuilder)]
#[owned_into(msg_api_client::dto::MsgMessageCategoryAddDto)]
#[ghosts(
    id: Default::default(),
    _current_ms: Default::default(),
    _current_user_id: robotech::cst::user_id_cst::SYS_OPERATOR_USER_ID.into(),
)]
#[serde(rename_all = "kebab-case")]
pub struct MsgMessageCategoryConfig {
    #[into(Some(~))]
    pub code: String,
    #[into(Some(~))]
    pub name: String,
    #[into(Some(~))]
    #[serde(default)]
    #[builder(default)]
    pub remark: Option<String>,
}

/// # 消息模板配置
///
/// 对应 `msg_message`。msg-svr 收到消息后按 `event_code` 查模板决定渲染与
/// 投递，模板不存在消息会被当作处理失败重投。
///
/// `event_source_id` / `category_id` 是关联 ID（由 `add` 事件来源与类别后从
/// 返回记录里取），因此本结构不做 o2o 直转，用 [`Self::to_add_dto`] 组装。
#[derive(Debug, Clone, Serialize, Deserialize, TypedBuilder)]
#[serde(rename_all = "kebab-case")]
pub struct MsgMessageConfig {
    /// 所属事件来源编码（须已注册，如 dtw 的 `event_source.location.code`）
    pub source_code: String,
    /// 消息类别（按 code 幂等注册，不存在时自动创建）
    pub category: MsgMessageCategoryConfig,
    /// 事件编码——发布 `MessageMqo` 时填的就是它，msg-svr 按它查模板
    pub event_code: String,
    /// 名称（唯一）
    pub name: String,
    /// 是否持久化
    #[serde(default = "persisted_default")]
    #[builder(default = persisted_default())]
    pub persisted: bool,
    /// 标题模板（`{var}` 占位符取消息 labels；为 null 时取 annotations.title）
    #[serde(default)]
    #[builder(default)]
    pub title_template: Option<String>,
    /// 内容模板（`{var}` 占位符取消息 labels；为 null 时取 annotations.content）
    #[serde(default)]
    #[builder(default)]
    pub content_template: Option<String>,
    /// 备注
    #[serde(default)]
    #[builder(default)]
    pub remark: Option<String>,
}

fn persisted_default() -> bool {
    true
}

impl MsgMessageConfig {
    /// # 组装 `MsgMessageAddDto`
    ///
    /// `event_source_id` 为事件来源记录 ID，`category_id` 为消息类别记录 ID
    /// （都由调用方 `add` 之后从返回记录里取）。组装结果可直接交给
    /// `msg_api_client.message.add(&dto)`。
    ///
    /// ## 参数
    ///
    /// - `event_source_id`: 事件来源 ID
    /// - `category_id`: 消息类别 ID（必填）
    pub fn to_add_dto(&self, event_source_id: u64, category_id: u64) -> MsgMessageAddDto {
        MsgMessageAddDto {
            event_source_id: Some(U64(event_source_id)),
            category_id: Some(U64(category_id)),
            event_code: Some(self.event_code.clone()),
            name: Some(self.name.clone()),
            persisted: Some(self.persisted),
            title_template: Some(self.title_template.clone()),
            content_template: Some(self.content_template.clone()),
            remark: Some(self.remark.clone()),
            _current_user_id: U64(SYS_OPERATOR_USER_ID),
            ..Default::default()
        }
    }
}
