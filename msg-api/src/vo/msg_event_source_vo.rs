use robotech::macros::vo;

#[vo]
pub struct MsgEventSourceVo {
    pub id: i64,
    pub code: String,
    pub name: String,
    pub remark: Option<String>,
    pub creator_id: i64,
    pub create_ms: i64,
    pub updator_id: i64,
    pub update_ms: i64,
}