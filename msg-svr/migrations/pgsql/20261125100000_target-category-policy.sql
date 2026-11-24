-- 目标类别升格为「业务规则载体」：是否允许广播、目标标识键；类别-渠道补默认标记
--
-- 规则由目标类别的 owner 在首次启动时注册上来（大屏 / 定位卡都由 dtw 注册），
-- msg 只按这些字段做通用展开，代码里不再出现 locator 这类业务字符串：
--   大屏（screen）：允许按类别广播，目标就是一条连接；
--   定位卡（locator）：禁止广播，卡号从消息 labels 按配置的键依次取。
-- 本迁移只加结构与存量回填，不改任何已上线迁移。

alter table public.msg_target_category
    add column allow_broadcast bool not null default true;

alter table public.msg_target_category
    add column target_keys varchar(100) null;

comment on column public.msg_target_category.allow_broadcast is
'是否允许广播
false 表示该类别必须给出具体 target_id：空目标（按类别广播）直接跳过并记原因
由类别 owner 启动注册时声明，msg 侧不做特例代码';

comment on column public.msg_target_category.target_keys is
'目标标识键
从消息 labels 取目标标识的键，逗号分隔按序取第一个有值者；值内逗号分隔表示多个目标
空表示只用配置的 target_id
@len 0..=100';

-- 存量回填：定位卡禁止广播，卡号从事件标签取
update public.msg_target_category
set allow_broadcast = false,
    target_keys = 'cardCodes,locatorCodes,locatorCode'
where code = 'locator';

alter table public.msg_target_category_channel
    add column is_default bool not null default false;

comment on column public.msg_target_category_channel.is_default is
'是否默认渠道
同一目标类别下最多一条：新建目标时优先预填该渠道
也是投递的回退出口：消息目标没有显式渠道行（msg_message_target_channel）时按它投
类别下一条默认都没标时投该类别全部绑定';

-- 表注释随语义一起更新：这张表从「仅 UI 预填」变成「显式绑定之外的回退出口」
comment on table public.msg_target_category_channel is
'目标类别渠道（默认渠道集）
新建消息目标时由 UI 预填，同时是投递的回退出口：消息目标没有显式渠道行时，
按这里 is_default 的渠道投递（一条都没标就投该类别全部绑定）
显式绑定 msg_message_target_channel 优先于这里';

-- 存量绑定每个类别保留一条默认（取最早建的那条）
update public.msg_target_category_channel c
set is_default = true
where c.id in (
    select min(x.id)
    from public.msg_target_category_channel x
    group by x.target_category_id
);
