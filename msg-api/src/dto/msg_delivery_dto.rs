use crate::dic::DeliverStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryDto {
    /// 消息ID
    pub message_id: i64,
    /// 指纹，可由业务触发时生成ID或直接采用业务ID，用于幂等去重
    pub business_id: i64,
    /// 投递状态
    #[db_default]
    pub deliver_status: DeliverStatus,
    /// 路由标签（一组结构化的 key-value，所有规则匹配、分组、路由、静默、维护窗口的过滤条件，都只针对 labels 做键值匹配或正则匹配，不碰任何自由文本）
    pub labels: Option<String>,
    /// 标注（放标题、详细描述、建议处理步骤这类可读文本，不参与任何匹配逻辑，只用于通知渲染和界面展示）
    pub annotations: Option<String>,
    /// 标题
    pub title: String,
    /// 内容
    pub content: String,
    /// 备注
    pub remark: Option<String>,
}