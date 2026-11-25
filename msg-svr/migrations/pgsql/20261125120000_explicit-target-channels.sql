-- 消息目标只认显式渠道绑定：取消投递时的类别默认渠道兜底
--
-- 写入侧已经在消息目标的新增/修改事务里同步 msg_message_target_channel，
-- 这里把存量没有绑定行的目标按类别默认渠道补齐，并同步表/列注释；
-- 迁移之后缓存构建不再回读 msg_target_category_channel。

insert into public.msg_message_target_channel (
    id, message_target_id, channel_id,
    creater_id, create_ms, updater_id, update_ms
)
select
    (select coalesce(max(id), 0) from public.msg_message_target_channel)
        + row_number() over (order by t.id, c.channel_id),
    t.id,
    c.channel_id,
    0,
    (extract(epoch from clock_timestamp()) * 1000)::bigint,
    0,
    (extract(epoch from clock_timestamp()) * 1000)::bigint
from public.msg_message_target t
join public.msg_target_category_channel c
    on c.target_category_id = t.target_category_id
where not exists (
    select 1 from public.msg_message_target_channel b
    where b.message_target_id = t.id
)
and (
    c.is_default
    or not exists (
        select 1 from public.msg_target_category_channel d
        where d.target_category_id = c.target_category_id and d.is_default
    )
);

comment on table public.msg_target_category_channel is
'目标类别渠道（默认渠道集）
新建消息目标时的预填模板：界面预填、服务端在自动注册时写入目标的显式绑定
投递不再回退到这里，只认 msg_message_target_channel';

comment on column public.msg_target_category_channel.is_default is
'是否默认渠道
同一目标类别下最多一条；新建消息目标时优先用它预填
投递侧没有「无显式绑定时回退」的逻辑';

comment on table public.msg_message_target_channel is
'消息目标渠道（投递渠道的唯一来源）
一条消息的某个目标走哪些渠道，只认这张表；同一目标至少一条、不重复
消息目标新增/修改时由服务端在同一事务里同步，删除目标时一并清理';

comment on column public.msg_message_target_channel.message_target_id is
'消息目标ID
关联 msg_message_target；与 channel_id 组成 AK，同一目标不会重复挂同一渠道';
