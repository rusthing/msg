/*==============================================================*/
/* 消息类别改为非空 + 新增消息级别表                              */
/*==============================================================*/

/*==============================================================*/
/* 消息类别改为非空                                              */
/*==============================================================*/

-- 改非空前先补齐历史数据：`category_id` 允许为空时注册的消息没有类别，
-- 直接 set not null 会失败。挂到现有类别里 id 最小的那个只是让数据满足约束，
-- 这类消息原本也进不了模板缓存，补完之后需要在消息中心确认类别是否正确。
update msg_message
   set category_id = (select min(id) from msg_message_category)
 where category_id is null
   and exists (select 1 from msg_message_category);

alter table msg_message
   alter column category_id set not null;

/*==============================================================*/
/* Table: msg_message_level                                     */
/*==============================================================*/
create table msg_message_level (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   remark               VARCHAR(50)          null,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_LEVEL primary key (id),
   constraint AK_CODE_MSG_MESSAGE_LEVEL unique (code),
   constraint AK_NAME_MSG_MESSAGE_LEVEL unique (name)
);

comment on table msg_message_level is
'消息级别';

comment on column msg_message_level.id is
'ID';

comment on column msg_message_level.code is
'编码';

comment on column msg_message_level.name is
'名称';

comment on column msg_message_level.remark is
'备注';

comment on column msg_message_level.creater_id is
'创建者ID';

comment on column msg_message_level.create_ms is
'创建时间戳';

comment on column msg_message_level.updater_id is
'更新者ID';

comment on column msg_message_level.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_message_level_PK                                  */
/*==============================================================*/
create unique index msg_message_level_PK on msg_message_level (
id
);
