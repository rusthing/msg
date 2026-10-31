use crate::dic::DeliverStatus;
use robotech::macros::crud_dto;

#[crud_dto]
pub struct MsgDeliveryDto {
    /// 事件编码（在应用中定义，事件触发时传递出来）
    /// @len 1..=50
    pub event_code: String,
    /// 消息ID
    pub message_id: u64,
    /// 事件来源ID
    pub event_source_id: u64,
    /// 消息类别ID
    pub message_category_id: u64,
    /// 指纹，可由业务触发时生成ID或直接采用业务ID，用于幂等去重
    pub business_id: u64,
    /// 业务触发时间戳（与business_id组成唯一约束，用于幂等去重）
    pub business_trigger_ms: u64,
    /// 接收时间戳（消息中心收到这条消息、开始落库处理的时刻，服务端时钟）
    pub receive_ms: u64,
    /// 首次接收时间戳（链路第一跳收到原始事件的时刻，由发布方透传；拿不到时为空）
    pub first_receive_ms: Option<u64>,
    /// 事件首次触发时间戳（发布方给的事件起点，不参与幂等；拿不到时为空）
    pub first_trigger_ms: Option<u64>,
    /// 投递状态
    #[db_default]
    pub deliver_status: DeliverStatus,
    /// 路由标签（一组结构化的 key-value，所有规则匹配、分组、路由、静默、维护窗口的过滤条件，都只针对 labels 做键值匹配或正则匹配，不碰任何自由文本）
    /// @len 0..
    pub labels: Option<String>,
    /// 标注（放标题、详细描述、建议处理步骤这类可读文本，不参与任何匹配逻辑，只用于通知渲染和界面展示）
    /// @len 0..
    pub annotations: Option<String>,
    /// 标题
    /// @len 1..=150
    pub title: String,
    /// 内容
    /// @len 1..
    pub content: String,
    /// 附加内容（模板渲染结果，格式不约定，可为空）
    /// @len 0..
    pub ext: Option<String>,
    /// 备注
    /// @len 0..=50
    pub remark: Option<String>,
}