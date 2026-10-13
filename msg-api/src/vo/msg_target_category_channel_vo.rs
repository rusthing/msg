use robotech::macros::vo;

#[vo]
pub struct MsgTargetCategoryChannelVo {
    pub id: u64,
    pub target_category_id: u64,
    pub channel_id: u64,
    pub creater_id: u64,
    pub create_ms: u64,
    pub updater_id: u64,
    pub update_ms: u64,
}