use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryChannelDto {
    pub target_category_id: u64,
    pub channel_id: u64,
}