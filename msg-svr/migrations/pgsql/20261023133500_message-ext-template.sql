/*==============================================================*/
/* 消息支持附加内容模板                                          */
/*==============================================================*/

-- 标题和内容改成投递的必备字段：模板为 null 时取发布方给的值，两边都空就拒收。
-- 附加内容（ext）是可选的第四块，格式不做约定，消费方自己判断怎么解。
--
-- 两列分开落：
--   msg_message.ext_template —— 模板，和 title_template / content_template 一起
--                               放在消息定义上，运营配置一次对所有投递生效；
--   msg_delivery.ext         —— 渲染结果快照，和 title / content 一起落库，
--                               排查时能看到当时到底发了什么。
--
-- 可空：没有附加内容就没有这一块，不给空串（空串会让消费方的 is_some() 失效）。
alter table msg_message add column ext_template text null;
alter table msg_delivery add column ext text null;

comment on column msg_message.ext_template is
'附加内容模板
渲染结果原样下发，格式不约定；为null时用发布方给的附加内容';

comment on column msg_delivery.ext is
'附加内容
模板渲染结果，格式不约定';
