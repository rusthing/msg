use robotech::macros::vo;

#[vo]
pub struct MsgTargetCategoryChannelVo {
    pub id: i64,
    pub target_category_id: i64,
    pub channel_id: i64,
    pub creator_id: i64,
    pub create_ms: i64,
    pub updator_id: i64,
    pub update_ms: i64,
}