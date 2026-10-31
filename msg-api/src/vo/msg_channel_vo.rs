use robotech::macros::vo;

#[vo]
pub struct MsgChannelVo {
    /// ID
    pub id: u64,
    /// 编码（同时是投递主题 `msg.delivery.{code}` 的末段）
    pub code: String,
    /// 名称
    pub name: String,
    /// 配置（按渠道自定义的 JSON，如 websocket 的 `{"token":""}`）
    pub options: Option<String>,
    /// 载荷模板：决定这个渠道的报文正文长什么样，为空时用渠道代码里的默认结构
    pub payload_template: Option<String>,
    /// 备注
    pub remark: Option<String>,
    /// 启用
    pub enabled: bool,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}