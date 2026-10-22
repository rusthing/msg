/*==============================================================*/
/* 初始化数据                                                    */
/*==============================================================*/

-- 消息必须挂在消息类别上：`msg_message.category_id`、`msg_delivery.message_category_id`
-- 都是非空外键，没有类别就既注册不了消息也建不了投递记录。
-- 先放一条「正常消息」作为兜底类别，业务方注册消息时未特别区分类别的都指向它。
--
-- id 固定为 1：id-worker 生成的是雪花 ID（长整型大数），不会与这个固定小 ID 冲突；
-- 用 `on conflict do nothing` 兼容已经手工插过同名类别的库，重复执行不报错。
-- 审计字段的 0 是 SYS_OPERATOR_USER_ID（系统操作员）。
insert into msg_message_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
) values (
   1,
   'normal',
   '正常消息',
   '默认类别：普通业务消息',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;
