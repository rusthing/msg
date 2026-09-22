use robotech::macros::vo;

#[vo]
pub struct MsgTargetCategoryVo {
    /// ID
    pub id: i64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 备注
    pub remark: Option<String>,
    /// 创建者ID
    pub creator_id: i64,
    /// 创建时间戳
    pub create_ms: i64,
    /// 更新者ID
    pub updator_id: i64,
    /// 更新时间戳
    pub update_ms: i64,
}