/// 消息主题前缀
///
/// 消息主题**不是代码拼出来的**：它配置在 `msg_message_queue.code` 里
/// （一个业务方一条队列，订阅与发布共用这一份）。约定取 `{前缀}.{队列名}`，
/// 例如 `msg.message.normal`（普通消息）、`msg.message.telemetry`（遥测）。
///
/// 这里保留前缀，供种子数据与排查时对照。
pub const MSG_MESSAGE_TOPIC_PREFIX: &str = "msg.message.";

/// 普通消息的消息主题
///
/// 业务方没有单独声明消息队列时都用它；msg-svr 启动时把它保底注册进
/// `msg_message_queue`，保证这个 subject 一定有订阅与 JetStream stream。
pub const MSG_MESSAGE_NORMAL_TOPIC: &str = "msg.message.normal";

/// 投递主题前缀
///
/// 投递主题**固定**为 `{前缀}{渠道类型}`，配置在 `msg_delivery_queue.code` 里
/// （一个渠道一条队列，订阅与发布共用这一份），例如 `msg.delivery.websocket`。
///
/// 队列表不存渠道 ID：末段就是 `msg_channel.channel_type`，反解见
/// [`delivery_topic_channel_type`]，写入校验见 `MsgDeliveryQueueSvc`。
///
/// 反过来，改渠道类型会同步改队列主题——`MsgChannelSvc::before_modify` 在
/// 同一个事务里把带旧类型的那条队列改成新主题，不需要人工跟着改。
pub const MSG_DELIVERY_TOPIC_PREFIX: &str = "msg.delivery.";

/// # 投递主题 → 渠道类型
///
/// 主题是投递队列与渠道之间**唯一**的关联：`msg.delivery.websocket` 的末段
/// 就是渠道类型 `websocket`。写入时用 [`MSG_DELIVERY_TOPIC_PREFIX`] 校验前缀，
/// 运行时（投递订阅）用本函数反解，不满足约定的主题返回 `None`，
/// 由调用方告警跳过——队列不会因为主题写错而把消息发到没人订阅的地方。
pub fn delivery_topic_channel_type(topic: &str) -> Option<&str> {
    topic
        .strip_prefix(MSG_DELIVERY_TOPIC_PREFIX)
        .filter(|channel_type| !channel_type.is_empty())
}

/// 目标标识的通配值：表示该目标类别下的全部目标
///
/// 大屏这类目标是动态接入的（哪块屏连上来才存在），发布方没法在登记消息时
/// 逐个写目标标识，用空串声明「发给这一类的全部目标」。投递时各实例把它展开到
/// 本地该类别下的连接。
///
/// `target_id` 是 `varchar(100)` 的非空列，空串在复合唯一约束
/// `(消息/投递, 类别, 目标)` 里能正常参与去重——不用再为广播补部分索引，
/// 也不会与真实标识撞车（编码不会为空）。
pub const TARGET_ID_ALL: &str = "";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_topic_channel_type_parses_suffix() {
        assert_eq!(
            delivery_topic_channel_type("msg.delivery.websocket"),
            Some("websocket")
        );
        // 只有前缀没有渠道：认不出渠道，按约定不合法
        assert_eq!(delivery_topic_channel_type("msg.delivery."), None);
        // 别的主题前缀不属于投递队列
        assert_eq!(delivery_topic_channel_type("msg.message.normal"), None);
        assert_eq!(delivery_topic_channel_type("websocket"), None);
        assert_eq!(delivery_topic_channel_type(""), None);
    }
}
