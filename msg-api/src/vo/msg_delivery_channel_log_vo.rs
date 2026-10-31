use crate::dic::DeliverChannelStatus;
use robotech::macros::vo;

#[vo]
pub struct MsgDeliveryChannelLogVo {
    /// ID
    pub id: u64,
    /// 投递渠道ID
    pub delivery_channel_id: u64,
    /// 事件首次触发时间戳（发布方给的事件起点；拿不到时为空）
    pub first_trigger_ms: Option<u64>,
    /// 首次接收时间戳（链路第一跳收到原始事件的时刻，从投递任务带下来；拿不到时为空）
    pub first_receive_ms: Option<u64>,
    /// 接收时间戳（渠道消费到投递任务、开始发送的时刻，服务端时钟）
    pub receive_ms: u64,
    /// 结束时间戳（本次发送尝试结束的时刻，服务端时钟）
    pub finish_ms: Option<u64>,
    /// 业务触发时间戳（本次投递对应的业务触发时间，与 msg_delivery.business_trigger_ms 同值）
    pub business_trigger_ms: u64,
    /// 投递状态
    pub deliver_channel_status: DeliverChannelStatus,
    /// 投递地址
    pub address: Option<String>,
    /// 投递详情
    pub detail: Option<String>,
    /// 创建者ID
    pub creater_id: u64,
    /// 创建时间戳
    pub create_ms: u64,
    /// 更新者ID
    pub updater_id: u64,
    /// 更新时间戳
    pub update_ms: u64,
}