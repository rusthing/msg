use robotech::macros::vo;

#[vo]
pub struct MsgMessageVo {
    /// ID
    pub id: u64,
    /// 消息类别ID
    pub category_id: u64,
    /// 消息队列ID
    pub message_queue_id: u64,
    /// 消息来源ID
    pub event_source_id: u64,
    /// 事件编码（在应用中定义，事件触发时传递出来）
    pub event_code: String,
    /// 名称
    pub name: String,
    /// 标题模板（为null的话从annotations中取title设置为标题）
    pub title_template: Option<String>,
    /// 内容模板（为null的话从annotations中取content设置为内容）
    pub content_template: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 启用
    pub enabled: bool,
    /// 创建者ID
    pub creator_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updator_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
    /// 消息队列
    pub msg_message_queue: MsgMessageQueueVo,
}