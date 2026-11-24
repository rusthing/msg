-- 渠道表加 channel_type（渠道类型），定位卡渠道的渠道类型定为 nats
--
-- 背景：`code` 原来一字段兼四职（类型、实例编码、投递主题末段、发送器分发键），
-- 一个类型只能配一条渠道。拆成：
--   channel_type  渠道类型，决定发送器与 options 结构（取值见 msg-api 的 ChannelCode）
--   code          实例编码，唯一，仍是投递主题 `msg.delivery.{code}` 的末段
-- 这样同一类型可以配多条实例（两个 NATS subject、两个短信网关、多个发件账号）。
--
-- 基线迁移 20261109000002_init-data.sql 已上线，不能直接改（checksum），所以用
-- 增量迁移；新库跑完基线再跑本迁移，结果一致。
--
-- ## 为什么不动 code
--
-- 存量四条的 code 就是类型名，看着像该一起改，但 code 同时是投递主题末段。
-- 改 code 等于换主题：`msg.delivery.locator` 上没消费完的消息会留在旧主题里，
-- 外部按 code 查 ID 建绑定的地方也得跟着改。而「定位卡渠道」是实例语义，
-- 保留 `locator` 恰好表达「这条实例是谁」，渠道类型另由 channel_type 表达。
-- 所以这里只补列、只定类型，code 与投递队列编码都不动。

alter table public.msg_channel add column channel_type varchar(20);

update public.msg_channel set channel_type = code;

alter table public.msg_channel alter column channel_type set not null;

comment on column public.msg_channel.channel_type is
'渠道类型
决定用哪个发送器、options 按哪套结构解析与校验，取值见 msg-api 的 ChannelCode（websocket / nats / sms / email）
与 code 分离：code 是实例编码（同时是投递主题末段），同一类型可以配多条渠道（不同 subject、不同网关、不同发件账号）
@len 1..=20';

comment on table public.msg_channel is
'渠道
投递的出口：channel_type 决定用哪个发送器与 options 的结构，code 是实例编码（同时是投递主题末段）
新增内置类型要同时改 ChannelCode、实现发送器并配一条投递队列；同一类型可以配多条实例';

-- 定位卡渠道：类型定为 nats，subject / 震动码 / 文案模板改由渠道配置决定
--
-- 行为没变：默认仍发 Core NATS 的 dtw.card.callback（dtw 订阅后写
-- queue_card_callback），code `locator` 与投递队列 `msg.delivery.locator` 保持不变。
-- options 写默认值，运营在控制台里能直接看到并改：
--   subject          Core NATS 主题
--   code             报文里的 code（定位卡 0 = 红蓝双灯同时震动）
--   messageTemplate  默认报文里的 message 文案，可用 {{title}} {{content}} 等变量

update public.msg_channel
set channel_type = 'nats',
    name = '定位卡渠道（NATS 下发）',
    remark = '定位卡下行：发 Core NATS，由 dtw 转 queue_card_callback',
    options = '{"subject":"dtw.card.callback","code":0,"messageTemplate":"{{title}} {{content}}"}'
where code = 'locator';

-- 队列只更新说明：编码仍必须等于 `msg.delivery.` + 渠道 code
update public.msg_delivery_queue
set name = '定位卡投递队列（NATS 转发）',
    remark = '定位卡下行：JetStream 队列组负载均衡，单实例发布到渠道配置的 NATS 主题'
where code = 'msg.delivery.locator';
