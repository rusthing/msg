/*==============================================================*/
/* 目标类别收口：恢复 target_id 非空 + 种三个默认目标类别          */
/*==============================================================*/

-- 上一版把 target_id 放开为可空，用 null 表示「该类别下的全部目标」。
-- 改成用 0 表示同样的语义：可空列在唯一约束里管不住重复（SQL 里 null
-- 互不相等），得靠部分唯一索引兜，而部分索引 PowerDesigner 生成不出来，
-- 只能手工补在建表脚本里，每次重新生成都要记得带回来。
-- 用 0 之后 (message_id, target_category_id, target_id) 这条普通唯一约束
-- 就够用，模型、建表脚本、DAO 判重三处不用再为广播开特例。
--
-- 回填放在收紧之前：上一版如果已经插过广播行，null 得先变成 0 才能 set not null。
update msg_message_target set target_id = 0 where target_id is null;
update msg_delivery_target set target_id = 0 where target_id is null;

alter table msg_message_target alter column target_id set not null;
alter table msg_delivery_target alter column target_id set not null;

comment on column msg_message_target.target_id is
'目标ID
目标类型为大屏时填0，表示该类别下的全部目标（大屏动态接入，不逐个登记）';

comment on column msg_delivery_target.target_id is
'目标ID
目标类型为大屏时填0，表示该类别下的全部目标（大屏动态接入，不逐个登记）';

-- 上一版为 null 值补的两条部分唯一索引，用 0 之后不再需要
drop index if exists ak_message_id_and_target_category_id_msg_message_target;
drop index if exists ak_delivery_id_and_target_category_id_msg_delivery_target;

/*==============================================================*/
/* 默认目标类别                                                  */
/*==============================================================*/

-- 目标类别是投递目标的字典：消息发给「谁」先落到类别上，再按类别找具体目标
-- （或按类别广播）。用户 / 角色 / 分组是平台级的固定口径，随库初始化种好，
-- 发布方登记消息目标时按 code 查 ID，不用自己建。
--
-- 「大屏」不在这里：它是动态接入的自定义目标（哪块屏连上来才算一个），
-- 由第一个用到它的发布方在启动时登记（dtw 就这么做），编码在发布方配置里。
--
-- id 固定为 1/2/3：id-worker 生成的是雪花 ID（长整型大数），不会与这些固定
-- 小 ID 冲突；`on conflict do nothing` 兼容已经手工插过同名类别的库，
-- 重复执行不报错。审计字段的 0 是 SYS_OPERATOR_USER_ID（系统操作员）。
insert into msg_target_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
) values
(
   1, 'user', '用户', '按用户投递：目标是平台用户ID',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
),
(
   2, 'role', '角色', '按角色投递：目标是角色ID，角色下的用户都收',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
),
(
   3, 'group', '分组', '按分组投递：目标是分组ID，分组内的用户都收',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;
