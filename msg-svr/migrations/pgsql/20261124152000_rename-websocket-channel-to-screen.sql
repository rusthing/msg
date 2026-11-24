-- 大屏渠道：实例编码 websocket → screen（渠道类型仍是 websocket）
--
-- 与定位卡渠道同样的命名口径：code 是**实例编码**，写业务身份（大屏、定位卡、
-- 短信主网关、通知发件箱），渠道类型另由 channel_type 表达（这条是 websocket）。
-- 原来 code 直接叫 websocket，把类型名当成实例名，配第二条 websocket 渠道时
-- 只能叫 websocket2 这类无意义的名字。
--
-- ## 影响面
--
-- 1. 投递主题随 code 变：`msg.delivery.websocket` → `msg.delivery.screen`。
--    大屏队列 `persisted = false`（长连接广播、不落盘），旧主题上没有可补消费的
--    积压；但订阅是按队列编码启动时建立的，**迁移后必须重启 msg-svr**。
-- 2. 类别-渠道绑定、消息-目标-渠道绑定都按 `channel_id` 存，不随编码变化。
-- 3. dtw 侧默认渠道编码同步改为 screen（`TARGET_CHANNEL_CODE_DEFAULT`）。生产
--    配置若显式写了 `channel-code = "websocket"`，要一并改成 screen，否则 dtw
--    启动时按旧编码查不到渠道，注册类别渠道绑定与消息目标会失败。

update public.msg_channel
set code = 'screen',
    name = '大屏渠道',
    remark = '大屏长连接：连接上报 targetCategory=screen，握手令牌在渠道配置里'
where code = 'websocket';

update public.msg_delivery_queue
set code = 'msg.delivery.screen',
    name = '大屏投递队列',
    remark = 'WebSocket 长连接广播，所有实例都收'
where code = 'msg.delivery.websocket';
