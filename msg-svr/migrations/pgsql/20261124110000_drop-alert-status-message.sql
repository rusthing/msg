-- 删除「报警状态」消息（alarm.alert.status）及其投递配置
--
-- 报警中心不再把存续期状态转成状态帧发往 msg.publish.screen：大屏的状态由
-- 触发 / 确认 / 结束通知承载，msg 模板多渲染出的 regionCount / durationMs /
-- reportMs 等字段前端并不解析，只剩每 5 秒一次的无用投递（还误绑了管理员
-- 角色的短信与邮件渠道）。
--
-- 基线迁移 20261109000002_init-data.sql 已上线，不能直接改（checksum），
-- 所以这里用增量迁移删除；新库跑完基线再跑本迁移，结果一致。
--
-- 按 event_code 定位而不是硬编码 ID，避免库里 ID 被调整过留下孤儿行。
-- 该消息 persisted=false，正常没有投递记录；这里仍按外键顺序把可能存在的
-- 历史投递记录一并清掉，避免 delete msg_message 被 restrict 挡住。

delete from public.msg_delivery_channel_log
where delivery_channel_id in (
    select id from public.msg_delivery_channel
    where deliver_target_id in (
        select id from public.msg_delivery_target
        where delivery_id in (
            select id from public.msg_delivery
            where message_id in (
                select id from public.msg_message
                where event_code = 'alarm.alert.status'
            )
        )
    )
);

delete from public.msg_delivery_channel
where deliver_target_id in (
    select id from public.msg_delivery_target
    where delivery_id in (
        select id from public.msg_delivery
        where message_id in (
            select id from public.msg_message
            where event_code = 'alarm.alert.status'
        )
    )
);

delete from public.msg_delivery_target
where delivery_id in (
    select id from public.msg_delivery
    where message_id in (
        select id from public.msg_message
        where event_code = 'alarm.alert.status'
    )
);

delete from public.msg_delivery
where message_id in (
    select id from public.msg_message
    where event_code = 'alarm.alert.status'
);

delete from public.msg_message_target_channel
where message_target_id in (
    select id from public.msg_message_target
    where message_id in (
        select id from public.msg_message
        where event_code = 'alarm.alert.status'
    )
);

delete from public.msg_message_target
where message_id in (
    select id from public.msg_message
    where event_code = 'alarm.alert.status'
);

delete from public.msg_message_queue
where code = 'msg.publish.screen';

delete from public.msg_message
where event_code = 'alarm.alert.status';
