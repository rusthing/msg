use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryChannelDto {
    /// 目标类别ID
    pub target_category_id: u64,
    /// 渠道ID
    pub channel_id: u64,
    /// 是否默认渠道
    ///
    /// 同一目标类别下最多一条。除了新建消息目标时预填，它还是投递的回退出口：
    /// 消息目标没有显式渠道行时按它投；类别下一条默认都没标就投全部绑定。
    pub is_default: bool,
}
