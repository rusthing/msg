use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgChannelDto {
    /// 名称
    pub name: String,
    /// 配置
    pub options: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 启用
    #[db_default]
    pub enabled: bool,
}