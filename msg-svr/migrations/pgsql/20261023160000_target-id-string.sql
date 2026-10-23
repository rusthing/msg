/*==============================================================*/
/* 目标标识改为字符串                                            */
/*==============================================================*/

-- 目标不再只有库内自增 ID：大屏这类目标的身份是它自己上报的编码，用户 / 角色 /
-- 分组也可能是外部系统的业务主键，统一用 varchar(100) 承载。
--
-- 「该类别下的全部目标」（上一版是 target_id = 0）改用**空串**表示：
-- 非空列在复合唯一约束里就能去重，不需要再为广播单独补部分索引；空串与任何
-- 真实标识都不会撞（编码里不会出现空串）。
--
-- 必须先把类型转过来再回填：int8 列上写 '' 直接报类型错误。
alter table msg_message_target
   alter column target_id type varchar(100) using target_id::varchar(100);

alter table msg_delivery_target
   alter column target_id type varchar(100) using target_id::varchar(100);

update msg_message_target set target_id = '' where target_id = '0';
update msg_delivery_target set target_id = '' where target_id = '0';

comment on column msg_message_target.target_id is
'目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）';

comment on column msg_delivery_target.target_id is
'目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）';
