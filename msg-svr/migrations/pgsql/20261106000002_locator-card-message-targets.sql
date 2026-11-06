/*==============================================================*/
/* 区域报警分流：定位卡渠道 + 升级事件 + SOS 新编码              */
/*==============================================================*/
--
-- ## 这一份改什么（跨仓契约 §4，已执行迁移 20261105000100 不改）
--
-- 20261105000100 给 7 类区域报警都挂了「全部大屏」这一个默认目标。按新的
-- 事件编码分流，这里用 DELETE/INSERT 调整目标，并补齐新事件：
--
-- - 两条 forbidden 首报改绑 `locator_card`（警告级，5s 内通知现场卡，不再
--   上大屏）；30s 未消警的升级事件改走 `screen`（严重级，大屏）；
-- - `gather` 在保留 screen 的同时加绑 `locator_card`（R1–R3 现场警示）；
-- - 其余区域事件（static/dwell/overcrowd/undercrowd）维持 screen 不变；
-- - SOS 统一新编码 `alarm.dtw.locator.sos`，绑 screen。
--
-- msg 侧无「级别路由」，纯粹按 event_code 决定投递目标；级别由 alarm/dtw
-- 侧的报警级别体现，不进这张表。
--
-- ## 旧数据直接删
--
-- 旧 SOS 编码 `dtw.locator.sos` 的消息行、它的投递目标以及旧编码产生的投递
-- 快照按用户决策**直接删**（旧数据可直接删，不写数据迁移、不做字段映射），
-- 不保留、不改名、不合并。删除后再插新编码，避免 `msg_message.name` /
-- `event_code` 唯一键撞车。
--
-- ## 消息行与目标的行 ID
--
-- 全部 `max(id)+row_number()` + `on conflict do nothing`：迁移先于报警中心
-- 启动执行，干净库上消息行由这里种下；若报警中心已按同名/同事件登记过，
-- 则复用那行，目标按 event_code 挂照样生效。

/*==============================================================*/
/* 1. 旧 SOS：消息行直接删                                      */
/*==============================================================*/

-- 旧数据直接删，不写数据迁移、不做字段映射，不保留/不改名/不合并。
--
-- 但 `msg_message` 被 `msg_delivery`、`msg_message_target` 以外键引用，且都是
-- `on delete restrict`：只删消息行会在跑过旧编码的库上被残留的投递记录挡住。
-- 所以按依赖顺序从叶到根，把旧编码产生的整条投递快照一并删掉（投递渠道日志
-- → 投递渠道 → 投递目标 → 投递 → 消息目标 → 消息）。范围只限废弃编码
-- `dtw.locator.sos`，不碰其它消息的数据；新库上这些表为空，全是 no-op。
delete from msg_delivery_channel_log
where delivery_channel_id in (
   select dc.id
   from msg_delivery_channel dc
   join msg_delivery_target dt on dt.id = dc.deliver_target_id
   join msg_delivery d on d.id = dt.delivery_id
   where d.message_id in (
      select id from msg_message where event_code = 'dtw.locator.sos'
   )
);

delete from msg_delivery_channel
where deliver_target_id in (
   select dt.id
   from msg_delivery_target dt
   join msg_delivery d on d.id = dt.delivery_id
   where d.message_id in (
      select id from msg_message where event_code = 'dtw.locator.sos'
   )
);

delete from msg_delivery_target
where delivery_id in (
   select id from msg_delivery
   where message_id in (
      select id from msg_message where event_code = 'dtw.locator.sos'
   )
);

delete from msg_delivery
where message_id in (
   select id from msg_message where event_code = 'dtw.locator.sos'
);

delete from msg_message_target
where message_id in (
   select id from msg_message where event_code = 'dtw.locator.sos'
);

-- 最后删消息行本身：上面的引用都清空后，restrict 不会再挡。
delete from msg_message where event_code = 'dtw.locator.sos';

/*==============================================================*/
/* 2. 定位卡目标：forbidden 首报改绑 + gather 加绑              */
/*==============================================================*/

-- 2a. forbidden 首报不再上大屏：删掉 20261105000100 挂的 screen 目标。
--     gather 的 screen 目标保留（R1–R3 仍要推大屏）。
delete from msg_message_target
where message_id in (
   select id from msg_message
   where event_code in (
      'alarm.dtw.region.enter.forbidden',
      'alarm.dtw.region.exit.forbidden'
   )
)
and target_category_id = (select id from msg_target_category where code = 'screen');

-- 2b. 三条事件挂到定位卡类别：两条 forbidden 首报（现场卡警告）+ gather
--     （R1–R3 现场警示）。target_id 空串 = 全部卡片，发送器落成 FFFFFFFF
--     广播；cardCodes 精确投卡由 dtw 订阅 dtw.card.callback 后处理，msg 侧
--     的 DeliveryMsg 不带 labels，无法在发送器里按卡号拆分。
insert into msg_message_target (
   id, message_id, target_category_id, target_id,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) from msg_message_target) + row_number() over (),
   m.id,
   tc.id,
   '',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_message m
join msg_target_category tc on tc.code = 'locator_card'
where m.event_code in (
   'alarm.dtw.region.enter.forbidden',
   'alarm.dtw.region.exit.forbidden',
   'alarm.dtw.region.gather'
)
and not exists (
   select 1 from msg_message_target b
   where b.message_id = m.id
     and b.target_category_id = tc.id
     and b.target_id = ''
)
on conflict do nothing;

/*==============================================================*/
/* 3. 新增消息行：两条升级事件 + SOS 新编码                     */
/*==============================================================*/

-- 3a. 30s 未消警升级事件（由 alarm 侧超时扫描产出，dtw 不直接判定）。
--     模板留空：与 7 类区域报警一致，文案由报警侧渲染后传过来。
insert into msg_message (
   id, event_source_id, category_id, name, event_code, persisted,
   title_template, content_template, ext_template, remark, enabled,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) from msg_message) + row_number() over (),
   s.id,
   c.id,
   v.name,
   v.event_code,
   true,
   null,
   null,
   null,
   v.remark,
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from (
   values
      ('区域禁入超时升级告警', 'alarm.dtw.region.enter.forbidden.escalated',
       '禁入首报 30 秒未消警升级：严重级，走大屏'),
      ('区域禁出超时升级告警', 'alarm.dtw.region.exit.forbidden.escalated',
       '禁出首报 30 秒未消警升级：严重级，走大屏')
) as v(name, event_code, remark)
join msg_event_source s on s.code = 'alarm'
join msg_message_category c on c.code = 'alarm'
on conflict do nothing;

-- 3b. SOS 新编码消息定义（附录 B 表 B.3：ALARM_TYPE=0 SOS 报警）。
--     ext 按附录 B 补 `ALARM_END_TIME` / `DESCRIBE`，并带报警状态机
--     `alertStatus`（接收方据此区分成立/确认/结束）；字符串值一律
--     `{{key:json}}`，缺值渲染成 null，整段永远是合法 JSON。
insert into msg_message (
   id, event_source_id, category_id, name, event_code, persisted,
   title_template, content_template, ext_template, remark, enabled,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message),
   s.id,
   c.id,
   '定位卡 SOS 报警',
   'alarm.dtw.locator.sos',
   true,
   '定位卡 {{locatorCode}} 触发 SOS 报警',
   '定位卡 {{locatorCode}} SOS 一键报警；所在区域 {{regionName:default(未知)}}；上报时间 {{triggerTimestamp:default(未知)}}',
   '{"alertId":{{alertId:json}},"eventCode":{{eventCode:json}},'
       || '"alertStatus":{{alertStatus:json}},"locatorCode":{{locatorCode:json}},'
       || '"ALARM_END_TIME":{{ALARM_END_TIME:json}},"DESCRIBE":{{DESCRIBE:json}}}',
   'SOS 一键报警（附录 B 表 B.3）：补报警结束时间与描述，走大屏',
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_event_source s
join msg_message_category c on c.code = 'alarm'
where s.code = 'alarm'
on conflict do nothing;

/*==============================================================*/
/* 4. 升级事件与 SOS 的默认目标：全部大屏                       */
/*==============================================================*/

insert into msg_message_target (
   id, message_id, target_category_id, target_id,
   creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) from msg_message_target) + row_number() over (),
   m.id,
   tc.id,
   '',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_message m
join msg_target_category tc on tc.code = 'screen'
where m.event_code in (
   'alarm.dtw.region.enter.forbidden.escalated',
   'alarm.dtw.region.exit.forbidden.escalated',
   'alarm.dtw.locator.sos'
)
and not exists (
   select 1 from msg_message_target b
   where b.message_id = m.id
     and b.target_category_id = tc.id
     and b.target_id = ''
)
on conflict do nothing;
