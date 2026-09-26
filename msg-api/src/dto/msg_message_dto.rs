use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageDto {
    /// 消息类别ID
    pub category_id: i64,
    /// 消息队列ID
    pub message_queue_id: i64,
    /// 消息来源ID
    pub event_source_id: i64,
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
    #[db_default]
    pub enabled: bool,
}