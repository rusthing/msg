use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageDto {
    /// 消息类别ID
    pub category_id: Option<i64>,
    /// 消息级别ID
    pub mes_id: i64,
    /// 消息来源ID
    pub source_id: Option<i64>,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 标题模板
    pub title_template: String,
    /// 内容模板
    pub content_template: String,
    /// 备注
    pub remark: Option<String>,
    /// 是否持久化
    #[db_default]
    pub persisted: bool,
    /// 启用
    #[db_default]
    pub enabled: bool,
}