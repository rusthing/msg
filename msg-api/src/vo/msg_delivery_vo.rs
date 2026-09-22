use crate::dic::DeliverStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryVo {
    /// ID
    pub id: i64,
    /// 消息ID
    pub message_id: i64,
    /// 业务ID，用于幂等去重，避免多次投递
    pub business_id: i64,
    /// 投递状态
    pub deliver_status: DeliverStatus,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
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