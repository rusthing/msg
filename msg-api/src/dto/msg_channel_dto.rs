use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgChannelDto {
    /// 编码（同时是投递主题 `msg.delivery.{code}` 的末段）
    pub code: String,
    /// 名称
    pub name: String,
    /// 配置（按渠道自定义的 JSON，如 websocket 的 `{"token":""}`）
    pub options: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 启用
    #[db_default]
    pub enabled: bool,
}