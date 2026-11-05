/*==============================================================*/
/* 报警状态消息：模板、队列与投递目标                            */
/*==============================================================*/
--
-- ## 这条消息为什么不归发布方注册
--
-- 报警中心把「报警存续期状态」直接发到 `msg.publish.screen`（BestEffort），
-- 不走 msg 的注册接口。消息中心要收下它、按模板渲染并投给大屏，三样东西都
-- 必须在库里先存在：
--
-- 1. 消息队列 `msg.publish.screen`：msg-svr 只订阅 `msg_message_queue` 里的
--    subject，没有这条记录发布出去没人消费；
-- 2. 消息模板 `alarm.alert.status`：订阅端按 `event_code` 查 `msg_message`
--    取模板，查不到直接按「未找到事件编码对应的消息模板」拒收；
-- 3. 投递目标：挂到大屏类别（空串 target_id 表示该类别下全部在线大屏）。
--
-- 模板变量来自报警中心状态消息的 labels：报警触发时的 labels（含 `regionName`
-- / `count`）原样带上，再补状态类字段（`alertStatus` / `alertLevelId` /
-- `regionCount` / `durationMs`）。区域名缺失时模板落 `未知区域`。

/*==============================================================*/
/* 1. 报警中心事件来源                                          */
/*==============================================================*/

-- 消息上的 event_source_id 非空；报警中心的来源通常已由它的同步钩子注册，
-- 这里只兜底补一条，编码相同就不重复插。
insert into msg_event_source (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_event_source),
   'alarm',
   '报警中心',
   '报警事件来源：事件策略与存续期状态',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 2. 大屏状态消息队列                                          */
/*==============================================================*/

-- 报警状态是「看一眼」的实时快照：丢一帧不影响报警本身，也不该落库重放，
-- 所以 `persisted = false`（Core NATS），与报警中心的 BestEffort 发布对齐。
insert into msg_message_queue (
   id, code, name, persisted, remark,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message_queue),
   'msg.publish.screen',
   '大屏状态消息队列',
   false,
   '报警存续期状态直推大屏',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 3. 报警状态消息模板                                          */
/*==============================================================*/

insert into msg_message (
   id, event_source_id, category_id, name, event_code, persisted,
   title_template, content_template, ext_template, remark, enabled,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message),
   s.id,
   c.id,
   '报警状态',
   'alarm.alert.status',
   false,
   '报警状态：{{eventCode:default(报警)}}',
   '区域{{regionName:default(未知区域)}} 事件{{eventCode}} 级别{{alertLevelId}} '
       || '人数{{regionCount:default(0)}} 持续{{durationMs:default(0)}}ms '
       || '状态{{alertStatus}}',
   '{"alertId":{{alertId}},"eventCode":{{eventCode:json}},'
       || '"alertStatus":{{alertStatus:json}},"alertLevelId":{{alertLevelId}},'
       || '"regionId":{{regionId:default(0)}},"regionCount":{{regionCount:default(0)}},'
       || '"durationMs":{{durationMs:default(0)}},"reportMs":{{reportMs}}}',
   '报警存续期状态推大屏：区域名、人数、事件、级别与持续时间',
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_event_source s
join msg_message_category c on c.code = 'normal'
where s.code = 'alarm'
on conflict do nothing;

/*==============================================================*/
/* 4. 投递目标（全部大屏）                                      */
/*==============================================================*/

insert into msg_message_target (
   id, message_id, target_category_id, target_id,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message_target),
   m.id,
   tc.id,
   '',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_message m
join msg_target_category tc on tc.code = 'screen'
where m.event_code = 'alarm.alert.status'
on conflict do nothing;
