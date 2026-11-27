-- 定位卡目标类别合并为「定位卡-卡号」（locator-card-no）
--
-- 以前按事件粒度拆两类：locator（禁入/禁出，单卡 locatorCode）与
-- locator-gather（聚集，清单 locatorCodes）。两者渠道相同，只是从 labels 取
-- 目标标识的键不同；消息中心本来就按类别上的 target_keys 通用展开，合成一类
-- 即可覆盖所有按卡号投递的事件，以后新增事件不必再建类别。
--
-- 基线迁移 20261109000002_init-data.sql 已上线，不能直接改；这里用增量迁移
-- 把旧类别的引用合并到新类别后删除。按 code / name 定位而不硬编码 ID：
-- 新库跑完基线再跑本迁移结果一致；dtw 下次启动按 code 回查同一行，不会重建。
--
-- 合并时按各表唯一键先清掉重复行（连带下游绑定与渠道日志），其余行只改外键：
-- 历史投递记录与消息目标上的显式渠道绑定都保留。

do $$
declare
    now_ms bigint := (extract(epoch from clock_timestamp()) * 1000)::bigint;
    new_id bigint;
    old_id bigint;
    gather_id bigint;
    src bigint;
begin
    select id into new_id
    from public.msg_target_category where code = 'locator-card-no' limit 1;
    select id into old_id
    from public.msg_target_category where code = 'locator' limit 1;
    select id into gather_id
    from public.msg_target_category
    where code = 'locator-gather' or name = '定位卡-聚集'
    order by (code = 'locator-gather') desc
    limit 1;

    -- 新类别不存在时先改造一个已有类别，避免丢掉它上面已挂的消息目标；
    -- 一个都没有才插（基线库里一定有 locator）。
    if new_id is null then
        new_id := coalesce(old_id, gather_id);
    end if;
    if new_id is null then
        insert into public.msg_target_category
            (code, name, remark, allow_broadcast, target_keys,
             creater_id, create_ms, updater_id, update_ms)
        values
            ('locator-card-no', '定位卡-卡号',
             '定位卡目标：卡号取自 labels.locatorCode / locatorCodes',
             false, 'locatorCodes,locatorCode', 0, now_ms, 0, now_ms)
        returning id into new_id;
    end if;

    -- 规则以 dtw 配置为准：编码改成通用的，名称不再绑定某个事件。
    update public.msg_target_category as c
    set code = 'locator-card-no',
        name = '定位卡-卡号',
        remark = '定位卡目标：卡号取自 labels.locatorCode / locatorCodes',
        allow_broadcast = false,
        target_keys = 'locatorCodes,locatorCode',
        update_ms = now_ms
    where c.id = new_id;

    foreach src in array array[old_id, gather_id] loop
        continue when src is null or src = new_id;

        -- 消息目标：同一消息 + 同一卡号在新类别下已有行时，旧行连同显式渠道
        -- 绑定一起删（新行已有自己的绑定）；其余只改外键。
        delete from public.msg_message_target_channel
        where message_target_id in (
            select s.id from public.msg_message_target as s
            where s.target_category_id = src
              and exists (
                  select 1 from public.msg_message_target as n
                  where n.target_category_id = new_id
                    and n.message_id = s.message_id
                    and n.target_id = s.target_id
              )
        );
        delete from public.msg_message_target as s
        where s.target_category_id = src
          and exists (
              select 1 from public.msg_message_target as n
              where n.target_category_id = new_id
                and n.message_id = s.message_id
                and n.target_id = s.target_id
          );
        update public.msg_message_target
        set target_category_id = new_id, update_ms = now_ms
        where target_category_id = src;

        -- 类别渠道：同一渠道已绑到新类别就不重复插，只把 is_default 并过去。
        update public.msg_target_category_channel as n
        set is_default = true
        where n.target_category_id = new_id
          and exists (
              select 1 from public.msg_target_category_channel as s
              where s.target_category_id = src and s.is_default
          );
        delete from public.msg_target_category_channel as s
        where s.target_category_id = src
          and exists (
              select 1 from public.msg_target_category_channel as n
              where n.target_category_id = new_id and n.channel_id = s.channel_id
          );
        update public.msg_target_category_channel
        set target_category_id = new_id, update_ms = now_ms
        where target_category_id = src;

        -- 历史投递记录：按 (delivery, target) 去重，冲突的连同渠道日志清掉；
        -- 不冲突的只改外键，留着给投递排障用。
        delete from public.msg_delivery_channel_log
        where delivery_channel_id in (
            select c.id from public.msg_delivery_channel as c
            where c.deliver_target_id in (
                select s.id from public.msg_delivery_target as s
                where s.target_category_id = src
                  and exists (
                      select 1 from public.msg_delivery_target as n
                      where n.target_category_id = new_id
                        and n.delivery_id = s.delivery_id
                        and n.target_id = s.target_id
                  )
            )
        );
        delete from public.msg_delivery_channel
        where deliver_target_id in (
            select s.id from public.msg_delivery_target as s
            where s.target_category_id = src
              and exists (
                  select 1 from public.msg_delivery_target as n
                  where n.target_category_id = new_id
                    and n.delivery_id = s.delivery_id
                    and n.target_id = s.target_id
              )
        );
        delete from public.msg_delivery_target as s
        where s.target_category_id = src
          and exists (
              select 1 from public.msg_delivery_target as n
              where n.target_category_id = new_id
                and n.delivery_id = s.delivery_id
                and n.target_id = s.target_id
          );
        update public.msg_delivery_target
        set target_category_id = new_id, update_ms = now_ms
        where target_category_id = src;

        delete from public.msg_target_category where id = src;
    end loop;
end $$;
