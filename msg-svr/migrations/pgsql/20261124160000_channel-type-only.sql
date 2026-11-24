-- 渠道收敛为只按「渠道类型」标识：删掉 code，唯一约束改挂 channel_type
--
-- 上一版（20261124150000）先把 channel_type 补进来，当时还留着 code 当实例编码；
-- 现在确认 code 与类型完全重复、业务名称放在 name 里就够了，所以把 code 去掉：
--   channel_type  渠道类型，唯一，决定发送器与 options 结构，
--                 同时是投递主题 `msg.delivery.{channel_type}` 的末段
--
-- 迁移链条保持线性：已上线的库跑完 150000 再跑本迁移；全新库也是
-- 基线 → 150000 → 本迁移，最终结构与模型一致。

-- 兜底：150000 之后若有人手工插过行，channel_type 可能为空，按 code 补上
update public.msg_channel
set channel_type = code
where channel_type is null or channel_type = '';

-- code 与类型重复，唯一约束换到 channel_type 上
alter table public.msg_channel drop constraint AK_CODE_MSG_CHANNEL;

alter table public.msg_channel
    add constraint AK_CHANNEL_TYPE_MSG_CHANNEL unique (channel_type);

alter table public.msg_channel drop column code;

comment on column public.msg_channel.channel_type is
'渠道类型
渠道唯一标识：决定用哪个发送器、options 按哪套结构解析与校验，也是投递主题 msg.delivery.{channel_type} 的末段
取值见 msg-api 的 ChannelCode（websocket / nats / sms / email），一个类型一条渠道
@len 1..=20';

comment on table public.msg_channel is
'渠道
投递的出口，由 channel_type 唯一标识：类型决定发送器与 options 的结构，同时是投递主题末段
新增内置类型要同时改 ChannelCode、实现发送器并配一条投递队列 msg.delivery.{channel_type}';

-- 大屏渠道：类型与主题都不变，只把业务名称改成「大屏渠道」
update public.msg_channel
set name = '大屏渠道',
    remark = '大屏长连接：连接上报 targetCategory=screen，令牌在渠道配置里'
where channel_type = 'websocket';

-- 定位卡队列：主题末段必须等于渠道类型，locator → nats
update public.msg_delivery_queue
set code = 'msg.delivery.nats'
where code = 'msg.delivery.locator';
