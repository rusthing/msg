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
    /// 载荷模板：决定这个渠道的报文正文长什么样，为空时用渠道代码里的默认结构
    ///
    /// 变量只有服务端给的这几个：`{eventCode}` `{title}` `{content}` `{ext}`
    /// `{firstReceiveMs}` `{businessTriggerMs}`，语法与消息模板一致
    /// （`{变量:过滤器}`，字符串要写成 `{title:json}`）。
    /// 改这里不用重启：渠道写操作会触发消息缓存全量重建。
    /// @len 0..
    pub payload_template: Option<String>,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
    /// 启用
    #[db_default]
    pub enabled: bool,
}