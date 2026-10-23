use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryChannelDto {
    /// 目标类别ID
    pub target_category_id: u64,
    /// 渠道ID
    pub channel_id: u64,
}