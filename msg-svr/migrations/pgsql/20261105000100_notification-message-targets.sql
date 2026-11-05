/*==============================================================*/
/* 区域报警通知：消息与默认投递目标                              */
/*==============================================================*/
--
-- ## 为什么需要这一份
--
-- 报警中心启动时只把事件策略登记成 `msg_message`（模板留空，文案由报警侧渲染），
-- **不登记投递目标**——目标被当作运营数据。新库里没有任何 `msg_message_target`
-- 行时，消息中心判定「没有登记任何投递目标」，直接按投递失败处理，大屏收不到
-- 任何区域报警。
--
-- 这一份给 7 类区域报警通知挂上大屏类别的默认目标（`target_id = ''` 表示该
-- 类别下的全部大屏）。消息行与类别也在这里兜底种好：干净库上迁移先跑、报警
-- 中心后起，只挂目标会因为没有消息行可挂而落空。
--
-- ## 与报警中心运行时登记的关系
--
-- 报警中心按 `event_code` 登记消息，撞唯一键即视为已登记并复用；`persisted`
-- 两边都是 true，不会产生纠正写。名称也沿用报警侧的策略名，避免两条唯一键
-- （`event_code`、`name`）互相打架。
--
-- 全部 `on conflict do nothing`：重复执行、或库里已有同名/同事件记录都不报错。

/*==============================================================*/
/* 1. 报警消息类别（运行时由报警中心登记，这里兜底）            */
/*==============================================================*/

insert into msg_message_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
)
select
   (select coalesce(max(id), 1) + 1 from msg_message_category),
   'alarm',
   '报警',
   '报警中心发出的报警通知',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
on conflict do nothing;

/*==============================================================*/
/* 2. 七类区域报警通知消息                                      */
/*==============================================================*/

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
      ('区域禁入报警', 'alarm.dtw.region.enter.forbidden', '人员进入禁止区域'),
      ('区域禁出报警', 'alarm.dtw.region.exit.forbidden', '人员离开禁止区域'),
      ('区域静止报警', 'alarm.dtw.region.static', '人员在区域内静止超时'),
      ('区域滞留报警', 'alarm.dtw.region.dwell', '人员在区域内滞留超时'),
      ('区域人员聚集报警', 'alarm.dtw.region.gather', '区域内人数超过聚集阈值'),
      ('区域超员报警', 'alarm.dtw.region.overcrowd', '区域内人数超过容量上限'),
      ('区域缺员报警', 'alarm.dtw.region.undercrowd', '区域内人数低于容量下限')
) as v(name, event_code, remark)
join msg_event_source s on s.code = 'alarm'
join msg_message_category c on c.code = 'alarm'
on conflict do nothing;

/*==============================================================*/
/* 3. 默认投递目标：全部大屏                                    */
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
   'alarm.dtw.region.enter.forbidden',
   'alarm.dtw.region.exit.forbidden',
   'alarm.dtw.region.static',
   'alarm.dtw.region.dwell',
   'alarm.dtw.region.gather',
   'alarm.dtw.region.overcrowd',
   'alarm.dtw.region.undercrowd'
)
on conflict do nothing;
