use robotech::macros::vo;

#[vo]
pub struct MsgEventSourceVo {
    pub id: u64,
    pub code: String,
    pub name: String,
    pub remark: Option<String>,
    pub creater_id: u64,
    pub create_ms: u64,
    pub updater_id: u64,
    pub update_ms: u64,
}