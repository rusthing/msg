use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgChannelDto {
    /// 编码（实例编码，同时是投递主题 `msg.delivery.{code}` 的末段）
    /// @len 1..=50
    pub code: String,
    /// 渠道类型：决定用哪个发送器、options 按哪套结构解析（见 `ChannelCode`）
    ///
    /// 与 code 分离后，同一类型可以配多条实例（不同 NATS subject、不同短信网关、
    /// 不同发件账号）。库里允许存未登记的类型，投递时会报「渠道未实现」。
    /// @len 1..=20
    pub channel_type: String,
    /// 名称
    /// @len 1..=50
    pub name: String,
    /// 配置（JSON，结构由 channel_type 决定；各类型的字段见发送器里的 Options 结构）
    /// @len 0..
    pub options: Option<String>,
    /// 载荷模板：决定这个渠道的报文正文长什么样，为空时用发送器内置的默认结构
    ///
    /// 变量只有服务端给的这几个：`{{eventCode}}` `{{title}}` `{{content}}` `{{ext}}`
    /// `{{firstReceiveMs}}` `{{businessTriggerMs}}`（NATS 渠道另有 `{{deviceId}}` `{{code}}`
    /// `{{message}}`，且 `message` 本身可用 `{{title}}` `{{content}}` 拼），语法与消息模板一致
    /// （`{{变量:过滤器}}`，字符串要写成 `{{title:json}}`）；`${变量}` 是等价别名。
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
