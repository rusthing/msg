/*==============================================================*/
/* 投递队列：按渠道配置投递主题与持久化方式                        */
/*==============================================================*/

-- 投递侧原来把主题写死在代码里（`msg.delivery.{渠道编码}`，Core NATS 广播），
-- 接短信 / 邮箱时改不了订阅方式。这张表把「渠道 → 投递队列」变成配置：
--
-- - `code` 就是投递主题（订阅与发布共用），换主题不用改代码；
-- - `persisted` 决定订阅方式：false 走 Core NATS 广播（websocket 必须如此——
--   连接在哪个实例，消息就得让所有实例都收到，各自查本地连接表），
--   true 走 JetStream + 队列组（短信 / 邮箱这类允许重投、需要负载均衡）。
--
-- 一个渠道只能有一条投递队列：没有队列配置的渠道既不会被订阅，也不该有任务
-- 发布出去（发布侧按渠道编码查这张表，查不到就记失败）。
create table msg_delivery_queue (
   id                   INT8                 not null,
   channel_id           INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   persisted            BOOL                 not null default false,
   remark               VARCHAR(50)          null,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_DELIVERY_QUEUE primary key (id),
   constraint AK_CODE_MSG_DELIVERY_QUEUE unique (code),
   constraint AK_NAME_MSG_DELIVERY_QUEUE unique (name),
   constraint AK_CHANNEL_ID_MSG_DELIVERY_QUEUE unique (channel_id)
);

comment on table msg_delivery_queue is
'投递队列
按渠道配置，消息中心据此订阅投递主题并决定是否持久化';

comment on column msg_delivery_queue.id is
'ID';

comment on column msg_delivery_queue.channel_id is
'渠道ID';

comment on column msg_delivery_queue.code is
'编码
投递主题，订阅与投递都用它';

comment on column msg_delivery_queue.name is
'名称';

comment on column msg_delivery_queue.persisted is
'是否持久化
持久化走 JetStream（落盘、重投、队列组负载均衡）；不持久化走 Core NATS 广播';

comment on column msg_delivery_queue.remark is
'备注';

comment on column msg_delivery_queue.creater_id is
'创建者ID';

comment on column msg_delivery_queue.create_ms is
'创建时间戳';

comment on column msg_delivery_queue.updater_id is
'更新者ID';

comment on column msg_delivery_queue.update_ms is
'更新时间戳';

create unique index msg_delivery_queue_PK on msg_delivery_queue (
id
);

create  index Relationship_18_FK on msg_delivery_queue (
channel_id
);

alter table msg_delivery_queue
   add constraint fk_channel_id__from__msg_channel foreign key (channel_id)
      references msg_channel (id)
      on delete restrict on update restrict;

/*==============================================================*/
/* websocket 渠道的投递队列                                      */
/*==============================================================*/

-- 主题保持原来的拼法（`msg.delivery.` + 渠道编码），已有部署的订阅不受影响；
-- persisted = false：websocket 是 at-most-once 广播，落 JetStream 反而会让
-- 「A 实例收任务、屏连在 B 实例」的消息被队列组吃掉。
--
-- id 固定为 1：id-worker 生成的是雪花 ID，不会与固定小 ID 冲突；
-- 渠道查不到时整条 select 不产出（上一版迁移种过 websocket 渠道，正常不会缺），
-- `on conflict do nothing` 兼容已手工配过同名队列的库。
insert into msg_delivery_queue (
   id, channel_id, code, name, persisted, remark,
   creater_id, create_ms, updater_id, update_ms
)
select
   1,
   c.id,
   'msg.delivery.websocket',
   'websocket 投递队列',
   false,
   '大屏长连接广播，所有实例都收',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_channel c
where c.code = 'websocket'
on conflict do nothing;
