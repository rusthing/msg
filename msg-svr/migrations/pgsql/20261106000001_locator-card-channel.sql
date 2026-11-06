/*==============================================================*/
/* 定位卡渠道：字典、绑定与投递队列                              */
/*==============================================================*/
--
-- ## 这一份放什么（跨仓契约 §4）
--
-- 定位卡下行：msg-svr 不直接连 RabbitMQ，而是把投递任务转成一条 Core NATS
-- 广播 `dtw.card.callback`，由 dtw 订阅后按懋特协议写 `queue_card_callback`。
-- 因此这里只是「让渠道能跑起来」的五条字典/配置数据，没有新表、没有新列：
--
-- 1. 消息类别 / 目标类别 / 渠道各加一条 `locator_card`；
-- 2. 目标类别只绑定到这条渠道（其它目标类别不受影响）；
-- 3. 给渠道配一条投递队列，主题固定 `msg.delivery.locator_card`。
--
-- ## 为什么 persisted = false
--
-- 与 websocket 同理：定位卡下行是「通知现场」的实时动作，不落库重放。
-- Core NATS 广播让每个 msg-svr 实例都收到任务、各自发一条下行；dtw 侧
-- 对同一 deviceId 的重复广播按协议去重即可。落 JetStream 反而会因队列组
-- 把消息摊到某个实例，没有必要。
--
-- 契约里「channel_id 唯一」体现在「一个渠道一条队列」：队列表不存渠道 ID，
-- 靠 `code` 唯一 + 末段即渠道编码保证，见 msg-svr/src/wkr/sub.rs。
--
-- id 用 `max(id)+1`：种子用的是个位数固定 ID，运行时 id-worker 生成的是
-- 雪花大数，不会撞；`on conflict do nothing` 兼容已手工插过同名记录的库。
-- 审计字段 0 = SYS_OPERATOR_USER_ID，时间戳取执行时刻。

/*==============================================================*/
/* 1. 消息类别：定位卡                                          */
/*==============================================================*/

insert into msg_message_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message_category),
   'locator_card',
   '定位卡',
   '定位卡下行通知的消息类别',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 2. 目标类别：定位卡                                          */
/*==============================================================*/

-- target_id 就是卡号；空串 = 该类别下全部卡片（发送器落成广播 FFFFFFFF）。
insert into msg_target_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_target_category),
   'locator_card',
   '定位卡',
   '定位卡目标：target_id 为卡号，空串广播全部卡片',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 3. 渠道：定位卡渠道                                          */
/*==============================================================*/

-- options / payload_template 都留空：定位卡不走 websocket 握手，也不吃
-- websocket 的帧模板；下行报文由 msg-svr/src/wkr/channel/locator_card.rs
-- 自己组装 {"deviceId","code","message"}。
insert into msg_channel (
   id, code, name, options, payload_template, remark, enabled,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_channel),
   'locator_card',
   '定位卡渠道',
   null,
   null,
   '定位卡下行：Core NATS 广播到 dtw.card.callback',
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 4. 目标类别 → 渠道绑定                                       */
/*==============================================================*/

insert into msg_target_category_channel (
   id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_target_category_channel),
   tc.id,
   c.id,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_target_category tc
join msg_channel c on c.code = 'locator_card'
where tc.code = 'locator_card'
  and not exists (
      select 1
      from msg_target_category_channel b
      where b.target_category_id = tc.id
        and b.channel_id = c.id
  )
on conflict do nothing;

/*==============================================================*/
/* 5. 投递队列：msg.delivery.locator_card                       */
/*==============================================================*/

insert into msg_delivery_queue (
   id, code, name, persisted, remark,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_delivery_queue),
   'msg.delivery.locator_card',
   '定位卡投递队列',
   false,
   '定位卡下行 Core NATS 广播，所有实例都收',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;
