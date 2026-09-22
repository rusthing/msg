use robotech::macros::vo;

#[vo]
pub struct MsgMessageVo {
    /// ID
    pub id: i64,
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
    pub persisted: bool,
    /// 启用
    pub enabled: bool,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}