/*==============================================================*/
/* DBMS name:      PostgreSQL 9.x                               */
/* Created on:     2026/9/22 17:15:59                           */
/*==============================================================*/


/*==============================================================*/
/* Table: msg_channel                                           */
/*==============================================================*/
create table msg_channel (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   options              TEXT                 null,
   remark               VARCHAR(50)          null,
   enabled              BOOL                 not null default true,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_CHANNEL primary key (id)
);

comment on table msg_channel is
'渠道';

comment on column msg_channel.id is
'ID';

comment on column msg_channel.code is
'编码';

comment on column msg_channel.name is
'名称';

comment on column msg_channel.options is
'配置';

comment on column msg_channel.remark is
'备注';

comment on column msg_channel.enabled is
'启用';

comment on column msg_channel.creator_id is
'创建人的用户ID';

comment on column msg_channel.create_ms is
'创建时间戳';

comment on column msg_channel.updator_id is
'修改人的用户ID';

comment on column msg_channel.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_channel_PK                                        */
/*==============================================================*/
create unique index msg_channel_PK on msg_channel (
id
);

/*==============================================================*/
/* Table: msg_delivery                                          */
/*==============================================================*/
create table msg_delivery (
   id                   INT8                 not null,
   message_id           INT8                 not null,
   business_id          INT8                 not null,
   deliver_status       INT2                 not null default 0,
   title                VARCHAR(150)         not null,
   content              TEXT                 not null,
   remark               VARCHAR(50)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_DELIVERY primary key (id),
   constraint AK_BUSINESS_ID_MSG_DELIVERY unique (business_id)
);

comment on table msg_delivery is
'投递';

comment on column msg_delivery.id is
'ID';

comment on column msg_delivery.message_id is
'消息ID';

comment on column msg_delivery.business_id is
'业务ID
用于幂等去重，避免多次投递';

comment on column msg_delivery.deliver_status is
'投递状态
0: 投递中
1: 投递部分成功
2: 投递全部成功
3: 投递全部失败
';

comment on column msg_delivery.title is
'标题';

comment on column msg_delivery.content is
'内容';

comment on column msg_delivery.remark is
'备注';

comment on column msg_delivery.creator_id is
'创建人的用户ID';

comment on column msg_delivery.create_ms is
'创建时间戳';

comment on column msg_delivery.updator_id is
'修改人的用户ID';

comment on column msg_delivery.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_delivery_PK                                       */
/*==============================================================*/
create unique index msg_delivery_PK on msg_delivery (
id
);

/*==============================================================*/
/* Index: Relationship_3_FK                                     */
/*==============================================================*/
create  index Relationship_3_FK on msg_delivery (
message_id
);

/*==============================================================*/
/* Table: msg_delivery_channel                                  */
/*==============================================================*/
create table msg_delivery_channel (
   id                   INT8                 not null,
   deliver_target_id    INT8                 not null,
   channel_id           INT8                 not null,
   deliver_channel_status INT2                 not null default 0,
   address              VARCHAR(80)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_DELIVERY_CHANNEL primary key (id),
   constraint AK_DELIVERY_TARGET_AND_CHANNEL_MSG_DELIVERY_CHANNEL unique (deliver_target_id, channel_id)
);

comment on table msg_delivery_channel is
'投递渠道';

comment on column msg_delivery_channel.id is
'ID';

comment on column msg_delivery_channel.deliver_target_id is
'投递目标ID';

comment on column msg_delivery_channel.channel_id is
'渠道ID';

comment on column msg_delivery_channel.deliver_channel_status is
'投递状态
0: 投递中
1: 投递成功
2: 投递失败
';

comment on column msg_delivery_channel.address is
'投递地址';

comment on column msg_delivery_channel.creator_id is
'创建人的用户ID';

comment on column msg_delivery_channel.create_ms is
'创建时间戳';

comment on column msg_delivery_channel.updator_id is
'修改人的用户ID';

comment on column msg_delivery_channel.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_delivery_channel_PK                               */
/*==============================================================*/
create unique index msg_delivery_channel_PK on msg_delivery_channel (
id
);

/*==============================================================*/
/* Index: Relationship_8_FK                                     */
/*==============================================================*/
create  index Relationship_8_FK on msg_delivery_channel (
deliver_target_id
);

/*==============================================================*/
/* Index: Relationship_9_FK                                     */
/*==============================================================*/
create  index Relationship_9_FK on msg_delivery_channel (
channel_id
);

/*==============================================================*/
/* Table: msg_delivery_channel_log                              */
/*==============================================================*/
create table msg_delivery_channel_log (
   id                   INT8                 not null,
   delivery_channel_id  INT8                 not null,
   deliver_channel_status INT2                 not null default 0,
   address              VARCHAR(80)          null,
   detail               TEXT                 null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_DELIVERY_CHANNEL_LOG primary key (id)
);

comment on table msg_delivery_channel_log is
'投递渠道日志';

comment on column msg_delivery_channel_log.id is
'ID';

comment on column msg_delivery_channel_log.delivery_channel_id is
'投递渠道ID';

comment on column msg_delivery_channel_log.deliver_channel_status is
'投递状态
0: 投递中
1: 投递成功
2: 投递失败
';

comment on column msg_delivery_channel_log.address is
'投递地址';

comment on column msg_delivery_channel_log.detail is
'投递详情';

comment on column msg_delivery_channel_log.creator_id is
'创建人的用户ID';

comment on column msg_delivery_channel_log.create_ms is
'创建时间戳';

comment on column msg_delivery_channel_log.updator_id is
'修改人的用户ID';

comment on column msg_delivery_channel_log.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_delivery_channel_log_PK                           */
/*==============================================================*/
create unique index msg_delivery_channel_log_PK on msg_delivery_channel_log (
id
);

/*==============================================================*/
/* Index: Relationship_10_FK                                    */
/*==============================================================*/
create  index Relationship_10_FK on msg_delivery_channel_log (
delivery_channel_id
);

/*==============================================================*/
/* Table: msg_delivery_target                                   */
/*==============================================================*/
create table msg_delivery_target (
   id                   INT8                 not null,
   delivery_id          INT8                 not null,
   target_category_id   INT8                 not null,
   target_id            INT8                 null,
   deliver_target_status INT2                 not null default 0,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_DELIVERY_TARGET primary key (id)
);

comment on table msg_delivery_target is
'投递目标';

comment on column msg_delivery_target.id is
'ID';

comment on column msg_delivery_target.delivery_id is
'投递ID';

comment on column msg_delivery_target.target_category_id is
'目标类别ID';

comment on column msg_delivery_target.target_id is
'目标ID';

comment on column msg_delivery_target.deliver_target_status is
'投递状态
0: 投递中
1: 投递成功
2: 投递失败
3: 目标已读';

comment on column msg_delivery_target.creator_id is
'创建人的用户ID';

comment on column msg_delivery_target.create_ms is
'创建时间戳';

comment on column msg_delivery_target.updator_id is
'修改人的用户ID';

comment on column msg_delivery_target.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_delivery_target_PK                                */
/*==============================================================*/
create unique index msg_delivery_target_PK on msg_delivery_target (
id
);

/*==============================================================*/
/* Index: Relationship_7_FK                                     */
/*==============================================================*/
create  index Relationship_7_FK on msg_delivery_target (
delivery_id
);

/*==============================================================*/
/* Index: Relationship_12_FK                                    */
/*==============================================================*/
create  index Relationship_12_FK on msg_delivery_target (
target_category_id
);

/*==============================================================*/
/* Table: msg_message                                           */
/*==============================================================*/
create table msg_message (
   id                   INT8                 not null,
   category_id          INT8                 null,
   mes_id               INT8                 not null,
   source_id            INT8                 null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   title_template       VARCHAR(150)         not null,
   content_template     TEXT                 not null,
   remark               VARCHAR(50)          null,
   enabled              BOOL                 not null default true,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE primary key (id),
   constraint AK_CODE_MSG_MESSAGE unique (code),
   constraint AK_NAME_MSG_MESSAGE unique (name)
);

comment on table msg_message is
'消息';

comment on column msg_message.id is
'ID';

comment on column msg_message.category_id is
'消息类别ID';

comment on column msg_message.mes_id is
'消息队列ID';

comment on column msg_message.source_id is
'消息来源ID';

comment on column msg_message.code is
'编码';

comment on column msg_message.name is
'名称';

comment on column msg_message.title_template is
'标题模板';

comment on column msg_message.content_template is
'内容模板';

comment on column msg_message.remark is
'备注';

comment on column msg_message.enabled is
'启用';

comment on column msg_message.creator_id is
'创建人的用户ID';

comment on column msg_message.create_ms is
'创建时间戳';

comment on column msg_message.updator_id is
'修改人的用户ID';

comment on column msg_message.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_PK                                        */
/*==============================================================*/
create unique index msg_message_PK on msg_message (
id
);

/*==============================================================*/
/* Index: Relationship_1_FK                                     */
/*==============================================================*/
create  index Relationship_1_FK on msg_message (
category_id
);

/*==============================================================*/
/* Index: Relationship_2_FK                                     */
/*==============================================================*/
create  index Relationship_2_FK on msg_message (
mes_id
);

/*==============================================================*/
/* Index: Relationship_13_FK                                    */
/*==============================================================*/
create  index Relationship_13_FK on msg_message (
source_id
);

/*==============================================================*/
/* Table: msg_message_category                                  */
/*==============================================================*/
create table msg_message_category (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   remark               VARCHAR(50)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_CATEGORY primary key (id)
);

comment on table msg_message_category is
'消息类别';

comment on column msg_message_category.id is
'ID';

comment on column msg_message_category.code is
'编码';

comment on column msg_message_category.name is
'名称';

comment on column msg_message_category.remark is
'备注';

comment on column msg_message_category.creator_id is
'创建人的用户ID';

comment on column msg_message_category.create_ms is
'创建时间戳';

comment on column msg_message_category.updator_id is
'修改人的用户ID';

comment on column msg_message_category.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_category_PK                               */
/*==============================================================*/
create unique index msg_message_category_PK on msg_message_category (
id
);

/*==============================================================*/
/* Table: msg_message_channel                                   */
/*==============================================================*/
create table msg_message_channel (
   id                   INT8                 not null,
   message_id           INT8                 not null,
   channel_id           INT8                 not null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_CHANNEL primary key (id),
   constraint AK_MSG_AND_CHANNEL_MSG_MESSAGE_CHANNEL unique (message_id, channel_id)
);

comment on table msg_message_channel is
'消息渠道';

comment on column msg_message_channel.id is
'ID';

comment on column msg_message_channel.message_id is
'消息ID';

comment on column msg_message_channel.channel_id is
'渠道ID';

comment on column msg_message_channel.creator_id is
'创建人的用户ID';

comment on column msg_message_channel.create_ms is
'创建时间戳';

comment on column msg_message_channel.updator_id is
'修改人的用户ID';

comment on column msg_message_channel.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_channel_PK                                */
/*==============================================================*/
create unique index msg_message_channel_PK on msg_message_channel (
id
);

/*==============================================================*/
/* Index: Relationship_5_FK                                     */
/*==============================================================*/
create  index Relationship_5_FK on msg_message_channel (
message_id
);

/*==============================================================*/
/* Index: Relationship_6_FK                                     */
/*==============================================================*/
create  index Relationship_6_FK on msg_message_channel (
channel_id
);

/*==============================================================*/
/* Table: msg_message_queue                                     */
/*==============================================================*/
create table msg_message_queue (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   persisted            BOOL                 not null default true,
   remark               VARCHAR(50)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_QUEUE primary key (id),
   constraint AK_CODE_MSG_MESSAGE_QUEUE unique (code),
   constraint AK_NAME_MSG_MESSAGE_QUEUE unique (name)
);

comment on table msg_message_queue is
'消息队列';

comment on column msg_message_queue.id is
'ID';

comment on column msg_message_queue.code is
'编码
用于订阅消息中间件队列的名称';

comment on column msg_message_queue.name is
'名称';

comment on column msg_message_queue.persisted is
'是否持久化';

comment on column msg_message_queue.remark is
'备注';

comment on column msg_message_queue.creator_id is
'创建人的用户ID';

comment on column msg_message_queue.create_ms is
'创建时间戳';

comment on column msg_message_queue.updator_id is
'修改人的用户ID';

comment on column msg_message_queue.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_queue_PK                                  */
/*==============================================================*/
create unique index msg_message_queue_PK on msg_message_queue (
id
);

/*==============================================================*/
/* Table: msg_message_source                                    */
/*==============================================================*/
create table msg_message_source (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   remark               VARCHAR(50)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_SOURCE primary key (id)
);

comment on table msg_message_source is
'消息来源';

comment on column msg_message_source.id is
'ID';

comment on column msg_message_source.code is
'编码';

comment on column msg_message_source.name is
'名称';

comment on column msg_message_source.remark is
'备注';

comment on column msg_message_source.creator_id is
'创建人的用户ID';

comment on column msg_message_source.create_ms is
'创建时间戳';

comment on column msg_message_source.updator_id is
'修改人的用户ID';

comment on column msg_message_source.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_source_PK                                 */
/*==============================================================*/
create unique index msg_message_source_PK on msg_message_source (
id
);

/*==============================================================*/
/* Table: msg_message_target                                    */
/*==============================================================*/
create table msg_message_target (
   id                   INT8                 not null,
   message_id           INT8                 not null,
   target_category_id   INT8                 not null,
   target_id            INT8                 null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_TARGET primary key (id),
   constraint AK_MESSAGE_AND_TARGET_CATEGORY_AN_MSG_MESSAGE_TARGET unique (message_id, target_category_id, target_id)
);

comment on table msg_message_target is
'消息目标';

comment on column msg_message_target.id is
'ID';

comment on column msg_message_target.message_id is
'消息ID';

comment on column msg_message_target.target_category_id is
'目标类别ID';

comment on column msg_message_target.target_id is
'目标ID';

comment on column msg_message_target.creator_id is
'创建人的用户ID';

comment on column msg_message_target.create_ms is
'创建时间戳';

comment on column msg_message_target.updator_id is
'修改人的用户ID';

comment on column msg_message_target.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_message_target_PK                                 */
/*==============================================================*/
create unique index msg_message_target_PK on msg_message_target (
id
);

/*==============================================================*/
/* Index: Relationship_4_FK                                     */
/*==============================================================*/
create  index Relationship_4_FK on msg_message_target (
message_id
);

/*==============================================================*/
/* Index: Relationship_11_FK                                    */
/*==============================================================*/
create  index Relationship_11_FK on msg_message_target (
target_category_id
);

/*==============================================================*/
/* Table: msg_target_category                                   */
/*==============================================================*/
create table msg_target_category (
   id                   INT8                 not null,
   code                 VARCHAR(50)          not null,
   name                 VARCHAR(50)          not null,
   remark               VARCHAR(50)          null,
   creator_id           INT8                 not null,
   create_ms            INT8                 not null,
   updator_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_TARGET_CATEGORY primary key (id)
);

comment on table msg_target_category is
'目标类别';

comment on column msg_target_category.id is
'ID';

comment on column msg_target_category.code is
'编码';

comment on column msg_target_category.name is
'名称';

comment on column msg_target_category.remark is
'备注';

comment on column msg_target_category.creator_id is
'创建人的用户ID';

comment on column msg_target_category.create_ms is
'创建时间戳';

comment on column msg_target_category.updator_id is
'修改人的用户ID';

comment on column msg_target_category.update_ms is
'修改时间戳';

/*==============================================================*/
/* Index: msg_target_category_PK                                */
/*==============================================================*/
create unique index msg_target_category_PK on msg_target_category (
id
);

alter table msg_delivery
   add constraint fk_message_id__from__msg_message foreign key (message_id)
      references msg_message (id)
      on delete restrict on update restrict;

alter table msg_delivery_channel
   add constraint fk_deliver_target_id__from__msg_delivery_target foreign key (deliver_target_id)
      references msg_delivery_target (id)
      on delete restrict on update restrict;

alter table msg_delivery_channel
   add constraint fk_channel_id__from__msg_channel foreign key (channel_id)
      references msg_channel (id)
      on delete restrict on update restrict;

alter table msg_delivery_channel_log
   add constraint fk_delivery_channel_id__from__msg_delivery_channel foreign key (delivery_channel_id)
      references msg_delivery_channel (id)
      on delete restrict on update restrict;

alter table msg_delivery_target
   add constraint fk_target_category_id__from__msg_target_category foreign key (target_category_id)
      references msg_target_category (id)
      on delete restrict on update restrict;

alter table msg_delivery_target
   add constraint fk_delivery_id__from__msg_delivery foreign key (delivery_id)
      references msg_delivery (id)
      on delete restrict on update restrict;

alter table msg_message
   add constraint fk_category_id__from__msg_message_category foreign key (category_id)
      references msg_message_category (id)
      on delete restrict on update restrict;

alter table msg_message
   add constraint fk_source_id__from__msg_message_source foreign key (source_id)
      references msg_message_source (id)
      on delete restrict on update restrict;

alter table msg_message
   add constraint fk_mes_id__from__msg_message_queue foreign key (mes_id)
      references msg_message_queue (id)
      on delete restrict on update restrict;

alter table msg_message_channel
   add constraint fk_message_id__from__msg_message foreign key (message_id)
      references msg_message (id)
      on delete restrict on update restrict;

alter table msg_message_channel
   add constraint fk_channel_id__from__msg_channel foreign key (channel_id)
      references msg_channel (id)
      on delete restrict on update restrict;

alter table msg_message_target
   add constraint fk_target_category_id__from__msg_target_category foreign key (target_category_id)
      references msg_target_category (id)
      on delete restrict on update restrict;

alter table msg_message_target
   add constraint fk_message_id__from__msg_message foreign key (message_id)
      references msg_message (id)
      on delete restrict on update restrict;

