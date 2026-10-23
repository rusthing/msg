use robotech::macros::vo;

#[vo]
pub struct MsgTargetCategoryChannelVo {
    /// ID
    pub id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 渠道ID
    pub channel_id: u64,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}