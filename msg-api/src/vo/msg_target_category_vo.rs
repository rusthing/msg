use robotech::macros::vo;

#[vo]
pub struct MsgTargetCategoryVo {
    /// ID
    pub id: u64,
    /// 编码
    pub code: String,
    /// 名称
    pub name: String,
    /// 备注
    pub remark: Option<String>,
    /// 是否允许广播（false 时目标必须给出具体 target_id）
    pub allow_broadcast: bool,
    /// 目标标识键（从消息 labels 取目标标识，逗号分隔）
    pub target_keys: Option<String>,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}
