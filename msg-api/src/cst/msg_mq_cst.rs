/// 消息主题
pub const MSG_MESSAGE_TOPIC: &str = "msg.message";

/// 投递主题前缀
///
/// 投递主题**不再是代码拼出来的**：它配置在 `msg_delivery_queue.code` 里
/// （一个渠道一条队列，订阅与发布共用这一份）。约定取 `{前缀}.{渠道编码}`，
/// 例如 `msg.delivery.websocket`，改主题只需改这张表。
///
/// 这里保留前缀，供种子数据与排查时对照。
pub const MSG_DELIVERY_TOPIC: &str = "msg.delivery";

/// websocket 渠道编码
///
/// 同时是 `msg_channel.code`、投递队列 `code` 的末段（`msg.delivery.websocket`）
/// 与 `ChannelSender::code` 的取值，三处必须一致。
pub const MSG_CHANNEL_CODE_WEBSOCKET: &str = "websocket";

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
