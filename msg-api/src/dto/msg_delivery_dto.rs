use crate::dic::DeliverStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryDto {
    /// 消息ID
    pub message_id: i64,
    /// 业务ID，用于幂等去重，避免多次投递
    pub business_id: i64,
    /// 投递状态
    #[db_default]
    pub deliver_status: DeliverStatus,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 备注
    pub remark: Option<String>,
}