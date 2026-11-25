use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgMessageTargetDto {
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标标识（用户 / 角色 / 分组 / 大屏的业务标识，最长 100 字符）
    ///
    /// 空串表示该类别下的全部目标：大屏这类动态接入的目标，发布方没法在
    /// 登记时知道有哪些屏，只能声明「发给大屏这一类」。
    /// @len 0..=100
    pub target_id: String,
    /// 这条目标要走的渠道 ID（可多条）
    ///
    /// 请求专用字段，不落在 `msg_message_target` 表上：服务层在同一个事务里
    /// 同步 `msg_message_target_channel`。新增/修改都接受；不传时服务端按
    /// 目标类别的默认渠道写入显式绑定（兼容发布方自动注册），类别也没配
    /// 渠道则报错，不会建出一条没有出口的目标。
    #[req_only]
    #[fill_by_svc]
    pub channel_ids: Vec<U64>,
}
