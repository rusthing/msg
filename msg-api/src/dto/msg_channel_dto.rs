use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgChannelDto {
    /// 编码（同时是投递主题 `msg.delivery.{code}` 的末段）
    /// @len 1..=50
    pub code: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 配置（按渠道自定义的 JSON，如 websocket 的 `{"token":""}`）
    /// @len 0..
    pub options: Option<String>,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
    /// 启用
    #[db_default]
    pub enabled: bool,
}