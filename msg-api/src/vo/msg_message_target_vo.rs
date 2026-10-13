use robotech::macros::vo;

#[vo]
pub struct MsgMessageTargetVo {
    pub id: u64,
    pub message_id: u64,
    pub target_category_id: u64,
    pub target_id: u64,
    pub creater_id: u64,
    pub create_ms: u64,
    pub updater_id: u64,
    pub update_ms: u64,
}