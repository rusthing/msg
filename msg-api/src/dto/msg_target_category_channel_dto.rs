use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgTargetCategoryChannelDto {
    pub target_category_id: i64,
    pub channel_id: i64,
}