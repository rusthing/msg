-- 消息模板的模拟投递开关
--
-- 控制「模拟产生的通知允不允许投递」。默认关，只有需要现场演示的消息才打开；
-- 关闭时带 `source=sim` / `simulated=true` 标记的通知在投递前被丢弃，不发到大屏。
-- 真实通知不带这两个标记，行为不受影响。

alter table msg_message
   add column if not exists sim_enabled boolean not null default false;

comment on column msg_message.sim_enabled is
'是否允许模拟通知投递
默认 false；关闭时带 simulated=true / source=sim 标记的通知会被丢弃';
