-- 给投递链路补时间戳
--
-- 一条消息从「业务触发」到「渠道发完」中间隔着三个服务，出问题时只能靠 trace
-- 日志把几段耗时拼起来。这里把几个时间点落到表上，查库就能算：
--
--   msg_delivery.trigger_ms → channel_log.finish_ms        端到端（事件发生到送达）
--   business_trigger_ms → msg_delivery.receive_ms          发布方到消息中心的延迟
--   msg_delivery.receive_ms → channel_log.receive_ms       消息中心拆任务、投递的延迟
--   channel_log.receive_ms → channel_log.finish_ms         渠道内的发送耗时
--
-- 这里面有两组容易混的时间，各占一列，谁也不能顶替谁：
--
--   business_trigger_ms / notify_ms  「这一次通知」的时间，跟着幂等键走，每次通知
--                                    都不同（报警重复提醒就是靠它区分）
--   trigger_ms                       「这件事什么时候开始发生」，同一条业务对象
--                                    的多条记录同值，不参与幂等，只用来算端到端
--
-- trigger_ms 是发布方可选给的：老数据和没接入的发布方没有这个值，所以两处都允许
-- 为空，不做回填——拿 now 补会让延迟恒为 0，拿 business_trigger_ms 补等于把「通知
-- 时间」冒充「事件时间」，两种都比空值更糟。
--
-- 其余新列都是 not null，上线时表里已经有数据，分三步走：先加可空列、回填、再收紧。
-- 回填取 create_ms：它就是服务端写库的时刻，对投递表来说等于「收到消息」，对渠道
-- 日志来说等于「这条日志写完」，即那次发送尝试的结束。

-- ── 投递表：接收时间（必填）与事件首次触发时间（可选） ──

alter table msg_delivery
   add column receive_ms INT8 null;

update msg_delivery
   set receive_ms = create_ms
 where receive_ms is null;

alter table msg_delivery
   alter column receive_ms set not null;

comment on column msg_delivery.receive_ms is
'接收时间戳
消息中心收到这条消息、开始落库处理的时刻
与 business_trigger_ms（发布方给的触发时间）之差就是发布方到消息中心的链路延迟';

alter table msg_delivery
   add column trigger_ms INT8 null;

comment on column msg_delivery.trigger_ms is
'事件首次触发时间戳
发布方给的事件起点，同一条业务对象的多条投递记录同值
与 business_trigger_ms 分工不同：那个是「这一次通知」的时间并参与幂等，这个是「这件事什么时候开始发生」且不参与幂等
与渠道日志的 finish_ms 之差才是端到端延迟；发布方拿不到事件起点时为空';

-- ── 渠道日志：触发 / 接收 / 结束 / 通知 ──

alter table msg_delivery_channel_log
   add column trigger_ms INT8 null;

alter table msg_delivery_channel_log
   add column receive_ms INT8 null;

alter table msg_delivery_channel_log
   add column finish_ms INT8 null;

alter table msg_delivery_channel_log
   add column notify_ms INT8 null;

-- 老数据没有「渠道收到任务」的时刻：日志本来就是发送结束后才写的，用 create_ms
-- 顶上，至少保证 finish_ms - receive_ms 不是负的、也不会凭空造出一段耗时。
update msg_delivery_channel_log
   set receive_ms = create_ms
 where receive_ms is null;

update msg_delivery_channel_log
   set finish_ms = create_ms
 where finish_ms is null;

-- 通知时间从投递记录上取：渠道日志 → 投递渠道 → 投递目标 → 投递。
-- 三张表的主键都在，join 一定走得通；取不到的孤儿行用接收时间兜底。
update msg_delivery_channel_log l
   set notify_ms = d.business_trigger_ms
  from msg_delivery_channel c
  join msg_delivery_target t on t.id = c.deliver_target_id
  join msg_delivery d on d.id = t.delivery_id
 where c.id = l.delivery_channel_id;

update msg_delivery_channel_log
   set notify_ms = coalesce(notify_ms, receive_ms)
 where notify_ms is null;

alter table msg_delivery_channel_log
   alter column receive_ms set not null;

alter table msg_delivery_channel_log
   alter column notify_ms set not null;

comment on column msg_delivery_channel_log.trigger_ms is
'事件首次触发时间戳
发布方给的事件起点，取自投递任务带下来的 trigger_ms
同一条业务对象的多条记录同值；发布方没给时为空，与 finish_ms 之差才是端到端延迟';

comment on column msg_delivery_channel_log.receive_ms is
'接收时间戳
渠道消费到这条投递任务的时刻，也就是本次发送尝试的开始
与 finish_ms 之差是渠道内的发送耗时';

comment on column msg_delivery_channel_log.finish_ms is
'结束时间戳
本次发送尝试结束（成功或失败）的时刻
与 receive_ms 之差是渠道内的发送耗时；发送中崩溃时为空，渠道状态停在「投递中」';

comment on column msg_delivery_channel_log.notify_ms is
'通知时间戳
本次投递对应的业务触发时间，取自投递任务带下来的 business_trigger_ms
与 msg_delivery.business_trigger_ms 同值，渠道日志可以脱离投递表单独算「通知到发送完成」的延迟
与 trigger_ms 的区别：这个是「这一次通知」的时间，每次通知都不同；那个是「事件首次发生」的时间';
