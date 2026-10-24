use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageDto {
    /// 事件来源ID
    pub event_source_id: u64,
    /// 消息类别ID
    pub category_id: u64,
    /// 事件编码（在应用中定义，事件触发时传递出来）
    /// @len 1..=50
    pub event_code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 是否持久化（如果不持久化，投递将不会保存到数据库）
    #[db_default]
    pub persisted: bool,
    /// 标题模板（`{var}` / `{var:json}` 取消息 labels 与 annotations；为 null 时用发布方给的标题）
    /// @len 0..=150
    pub title_template: Option<String>,
    /// 内容模板（`{var}` / `{var:json}` 取消息 labels 与 annotations；为 null 时用发布方给的内容）
    /// @len 0..
    pub content_template: Option<String>,
    /// 附加内容模板（渲染结果原样下发，格式不约定；为 null 时用发布方给的附加内容）
    /// @len 0..
    pub ext_template: Option<String>,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
    /// 启用
    #[db_default]
    pub enabled: bool,
}