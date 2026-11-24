-- 渠道标识统一为「渠道类型」：删除 code，原 code 列改名为 channel_type
--
-- 背景：`code` 原来一字段兼四职（渠道类型、投递主题末段、发送器分发键、业务编码），
-- 但渠道本来就只有类型这一个身份：websocket / nats / sms / email。业务命名放在
-- `name` 里（大屏渠道、定位卡渠道），不需要再有一列编码。所以：
--   channel_type  渠道类型，唯一，决定发送器与 options 结构
--                 同时是投递主题 `msg.delivery.{channel_type}` 的末段
--
-- 存量四条的 code 取值（websocket / locator / sms / email）本来就是类型，
-- 直接改名即可，只有定位卡的 `locator` 要按实际行为改成 `nats`。
--
-- 基线迁移 20261109000002_init-data.sql 已上线，不能直接改（checksum），所以用
-- 增量迁移；新库跑完基线再跑本迁移，结果一致。

alter table public.msg_channel rename column code to channel_type;

alter table public.msg_channel alter column channel_type type varchar(20);

alter table public.msg_channel
    rename constraint AK_CODE_MSG_CHANNEL to AK_CHANNEL_TYPE_MSG_CHANNEL;

comment on column public.msg_channel.channel_type is
'渠道类型
渠道唯一标识：决定用哪个发送器、options 按哪套结构解析与校验，也是投递主题 msg.delivery.{channel_type} 的末段
取值见 msg-api 的 ChannelCode（websocket / nats / sms / email），一个类型一条渠道
@len 1..=20';

comment on table public.msg_channel is
'渠道
投递的出口，由 channel_type 唯一标识：类型决定发送器与 options 的结构，同时是投递主题末段
新增内置类型要同时改 ChannelCode、实现发送器并配一条投递队列 msg.delivery.{channel_type}';

-- 定位卡渠道：类型由 locator 改成 nats，参数改由渠道配置决定
--
-- 行为没变：默认仍发 Core NATS 的 dtw.card.callback（dtw 订阅后写
-- queue_card_callback）。options 写默认值，运营在控制台里能直接看到并改：
--   subject          Core NATS 主题
--   code             报文里的 code（定位卡 0 = 红蓝双灯同时震动）
--   messageTemplate  默认报文里的 message 文案，可用 {{title}} {{content}} 等变量

update public.msg_channel
set channel_type = 'nats',
    name = '定位卡渠道（NATS 下发）',
    remark = '定位卡下行：Core NATS 主题发布，由 dtw 转投递队列',
    options = '{"subject":"dtw.card.callback","code":0,"messageTemplate":"{{title}} {{content}}"}'
where channel_type = 'locator';

-- 投递主题末段必须等于渠道类型（`delivery_topic_channel_type` 按此反解），
-- 这里同步改队列编码；订阅会在重启后按新主题重建。
update public.msg_delivery_queue
set code = 'msg.delivery.nats',
    name = '定位卡投递队列（NATS 转发）',
    remark = '定位卡下行：JetStream 队列组负载均衡，单实例发布到渠道配置的 NATS 主题'
where code = 'msg.delivery.locator';

-- 大屏渠道只改名称：类型 websocket 与投递主题 msg.delivery.websocket 都不变
update public.msg_channel
set name = '大屏渠道',
    remark = '大屏长连接：连接上报 targetCategory=screen，握手令牌在渠道配置里'
where channel_type = 'websocket';
