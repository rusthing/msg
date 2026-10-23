/*==============================================================*/
/* 消息目标支持按类别广播                                        */
/*==============================================================*/

-- 目标 ID 允许为空：为空表示「该目标类别下的全部目标」。
--
-- 大屏这类目标是动态接入的（每块屏连上来才存在，还可能在任意实例上），
-- 发布方不可能在登记消息时逐个登记，只能声明「这条消息发给大屏这一类」，
-- 由投递时各实例展开到本地连接。用户 / 角色 / 分组这类目标仍然逐条登记。
--
-- 投递目标表同步放开：广播投递没有具体目标可记，只能记一条 target_id 为空
-- 的行，表示「这次投递覆盖了该类别下的全部目标」。
alter table msg_message_target alter column target_id drop not null;
alter table msg_delivery_target alter column target_id drop not null;

comment on column msg_message_target.target_id is
'目标ID（为空表示该目标类别下的全部目标，即按类别广播）';

comment on column msg_delivery_target.target_id is
'目标ID（为空表示该目标类别下的全部目标，即按类别广播）';

-- 上面两条唯一约束在 target_id 为空时管不住重复：SQL 里 null 互不相等，
-- (消息, 类别, null) 可以插任意多条，发布方每次重启都会多插一条广播行，
-- 缓存里就会出现多条重复目标、一条消息投递多次。
--
-- 因此各补一条部分唯一索引，只约束 target_id 为空的那部分。
-- PowerDesigner 不支持部分索引，这两条是在生成建表脚本后手工补的，
-- 重新生成 create-table-pgsql.sql 时要一并带回来。
--
-- 索引名必须保持 ak_{列名用_and_连接}_{表名} 的格式：robotech 识别重复键
-- 时是按 PostgreSQL 报出的约束名回查字段列表的（unique_keys_utils.rs），
-- 名字对不上会被当成「获取 unique 字段列表错误」，接口直接 500 而不是
-- 返回 RTW00001。两个 DAO 里也按同样规则各补了一条唯一键声明。
create unique index ak_message_id_and_target_category_id_msg_message_target
   on msg_message_target (message_id, target_category_id)
   where target_id is null;

create unique index ak_delivery_id_and_target_category_id_msg_delivery_target
   on msg_delivery_target (delivery_id, target_category_id)
   where target_id is null;
