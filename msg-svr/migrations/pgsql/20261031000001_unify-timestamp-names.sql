-- 统一时间戳命名，并补上链路首收时间
--
-- 这次只做两件事：改名、补列，不改任何行为。老数据不需要回填。
--
-- ── 1. 同一个概念只留一个名字 ──
--
-- 渠道日志的 notify_ms 与投递表的 business_trigger_ms 是同一个值（投递任务
-- 原样带下来的那个），一个概念两个名字，看库的人分不清谁是谁。统一成
-- business_trigger_ms —— 它同时是幂等键的组成部分，也是发布报文里的字段名。
--
-- trigger_ms 在这两张表里的含义都是「事件首次触发」（同一条业务对象不随通知
-- 次数变化），与报警中心 alarm_alert.first_trigger_ms 是同一个量，改同名。
-- 与 alarm_event_log.trigger_ms 区分开：那一列是「单次上报的事件时间」。
--
-- ── 2. 补 first_receive_ms ──
--
-- 之前链路上只有 first_trigger_ms，它来自发布方/终端时钟：卡片没对时、离线
-- 补传时，端到端延迟会被算成几分钟甚至负数。first_receive_ms 是链路第一跳
-- 收到原始事件的服务端时刻，由发布方透传，端到端延迟只信它。
--
-- 两列都允许为空：没接入的发布方给不出这个值。不做回填——拿 create_ms 顶上
-- 会把「消息中心收到」冒充「链路首收」，等于把一段真实存在的耗时抹成 0。

-- ── 投递表 ──

alter table msg_delivery rename column trigger_ms to first_trigger_ms;

alter table msg_delivery add column first_receive_ms INT8 null;

comment on column msg_delivery.business_trigger_ms is
'业务触发时间戳
与业务ID组合唯一约束，用于幂等去重，避免多次投递
是「这一次通知」的时间，每次通知都不同（报警的重复提醒靠它区分）；事件起点看 first_trigger_ms
取值跟着幂等键走：事件触发的通知用事件本身的时间（重投/重放不变），人为操作或定时提醒用发起时刻';

comment on column msg_delivery.receive_ms is
'接收时间戳
消息中心收到这条消息、开始落库处理的时刻（服务端时钟）
与 business_trigger_ms（发布方给的触发时间）之差就是发布方到消息中心的链路延迟';

comment on column msg_delivery.first_receive_ms is
'首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），由发布方透传下来，不是本服务打的点
与 first_trigger_ms 的区别是「谁的时钟」：那个是业务事实（事件发生了），这个是对时事实（我们的服务什么时候知道的）
发布方没接入或拿不到时为空；要算不受终端时钟偏差影响的端到端延迟，只能用这一列';

comment on column msg_delivery.first_trigger_ms is
'事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），同一条业务对象的多条投递记录同值
与 business_trigger_ms 分工不同：那个是「这一次通知」的时间并参与幂等，这个是「这件事什么时候开始发生」且不参与幂等
与渠道日志的 finish_ms 之差是「事件发生 → 送达」，但这个差值含发布方时钟偏差；发布方拿不到事件起点时为空';

-- ── 渠道日志 ──

alter table msg_delivery_channel_log rename column trigger_ms to first_trigger_ms;

alter table msg_delivery_channel_log rename column notify_ms to business_trigger_ms;

alter table msg_delivery_channel_log add column first_receive_ms INT8 null;

comment on column msg_delivery_channel_log.first_trigger_ms is
'事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），取自投递任务带下来的 first_trigger_ms
同一条业务对象的多条记录同值；发布方没给时为空，与 finish_ms 之差是「事件发生 → 送达」，含发布方时钟偏差';

comment on column msg_delivery_channel_log.first_receive_ms is
'首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），从投递任务带下来
渠道日志可以脱离投递表单独算「链路首收 → 送达」的端到端延迟，且不受终端时钟偏差影响
发布方没给时为空';

comment on column msg_delivery_channel_log.receive_ms is
'接收时间戳
渠道消费到这条投递任务的时刻，也就是本次发送尝试的开始（服务端时钟）
与 finish_ms 之差是渠道内的发送耗时';

comment on column msg_delivery_channel_log.finish_ms is
'结束时间戳
本次发送尝试结束（成功或失败）的时刻（服务端时钟）
与 receive_ms 之差是渠道内的发送耗时；发送中崩溃时为空，渠道状态停在「投递中」';

comment on column msg_delivery_channel_log.business_trigger_ms is
'业务触发时间戳
本次投递对应的业务触发时间，取自投递任务带下来的 business_trigger_ms
与 msg_delivery.business_trigger_ms 同值，渠道日志可以脱离投递表单独算「通知到发送完成」的延迟
与 first_trigger_ms 的区别：这个是「这一次通知」的时间，每次通知都不同；那个是「事件首次发生」的时间';
