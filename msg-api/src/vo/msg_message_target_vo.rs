use robotech::macros::vo;

#[vo]
pub struct MsgMessageTargetVo {
    /// ID
    pub id: u64,
    /// 消息ID
    pub message_id: u64,
    /// 目标类别ID
    pub target_category_id: u64,
    /// 目标ID（用户 / 角色 / 分组 / 大屏的 ID），0 表示该类别下的全部目标
    pub target_id: u64,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}