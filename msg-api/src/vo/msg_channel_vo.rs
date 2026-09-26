use robotech::macros::vo;

#[vo]
pub struct MsgChannelVo {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub options: Option<String>,
    pub remark: Option<String>,
    pub enabled: bool,
    pub creator_id: i64,
    pub create_ms: i64,
    pub updator_id: i64,
    pub update_ms: i64,
}