-- 新增迁移，不改历史文件
--
-- 为什么单开一个文件：`20261109000000_create-table.sql` 已在生产 msgrs 应用过，
-- 再改它会 checksum mismatch 让 msgrs_svr 直接起不来（见「部署依赖与迁移记录」第 25 行）。
-- 搬进来的两样：
--   1) `msg_message_target_channel`（投递渠道的唯一来源，S0 把渠道从类别下移到目标）；
--   2) 六张 adm-rs 投影表（S1，dbz 同步写入，禁止在 msg 侧改）。

/*==============================================================*/
/* Table: msg_message_target_channel                            */
/*==============================================================*/
create table msg_message_target_channel (
   id                   INT8                 not null,
   message_target_id    INT8                 not null,
   channel_id           INT8                 not null,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_MESSAGE_TARGET_CHANNEL primary key (id),
   constraint AK_MESSAGE_TARGET_ID_AND_CHANNEL_ID_MSG_MESSAGE_TARGET_CHANNEL unique (message_target_id, channel_id)
);

comment on table msg_message_target_channel is
'消息目标渠道
投递渠道的唯一来源：一条消息的某个目标走哪些渠道，只认这张表
目标至少要有一条渠道；为零时该目标不产生投递任务，投递侧报错跳过、缓存刷新时报错';

comment on column msg_message_target_channel.id is
'ID';

comment on column msg_message_target_channel.message_target_id is
'消息目标ID
关联 msg_message_target；与 channel_id 组成 AK，同一目标不会重复挂同一渠道';

comment on column msg_message_target_channel.channel_id is
'渠道ID
关联 msg_channel；渠道 enabled=false 时等价于没配';

comment on column msg_message_target_channel.creater_id is
'创建者ID';

comment on column msg_message_target_channel.create_ms is
'创建时间戳';

comment on column msg_message_target_channel.updater_id is
'更新者ID';

comment on column msg_message_target_channel.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_message_target_channel_PK                         */
/*==============================================================*/
create unique index msg_message_target_channel_PK on msg_message_target_channel (
id
);

/*==============================================================*/
/* Index: msg_message_target_channel_message_target_id          */
/*==============================================================*/
create  index msg_message_target_channel_message_target_id on msg_message_target_channel (
message_target_id
);

/*==============================================================*/
/* Index: msg_message_target_channel_channel_id                 */
/*==============================================================*/
create  index msg_message_target_channel_channel_id on msg_message_target_channel (
channel_id
);

-- 原基线里 `msg_target_category_channel` 的表注释是「目标类别渠道」，
-- 语义已降级，这里覆盖一次。
comment on table msg_target_category_channel is
'目标类别渠道（默认渠道集）
只用于新建消息目标时给 UI 预填，不参与投递
投递只认 msg_message_target_channel';


/*==============================================================*/
/* Table: msg_user                                              */
/*==============================================================*/
create table msg_user (
   id                   INT8                 not null,
   person_id            INT8                 null,
   name                 VARCHAR(50)          not null,
   nick_name            VARCHAR(50)          null,
   real_name            VARCHAR(200)         null,
   mobile               VARCHAR(20)          null,
   email                VARCHAR(50)          null,
   expire_ms            INT8                 null,
   enabled              BOOL                 not null default true,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_USER primary key (id)
);

comment on table msg_user is
'用户投影（由同步写入，禁止在 msg 侧修改）
adm_user 的只读副本，为的是 adm 不可用时 msg 仍能发短信/邮件
status 不投影：锁定与停用之外的状态不影响投递，可达性只看 enabled + expire_ms';

comment on column msg_user.id is
'ID
同 adm_user.id';

comment on column msg_user.person_id is
'人员ID
关联 msg_person；为空表示纯账号，地址兜底时取不到 person';

comment on column msg_user.name is
'登录名
称呼兜底链的最末一级';

comment on column msg_user.nick_name is
'昵称
称呼兜底链的第二级';

comment on column msg_user.real_name is
'实名
称呼兜底链的第一级';

comment on column msg_user.mobile is
'手机号
短信地址首选；为空时降级取 msg_person.mobile';

comment on column msg_user.email is
'邮箱
邮件地址首选；为空时降级取 msg_person.email';

comment on column msg_user.expire_ms is
'过期时间戳
为空表示不过期；与 enabled 一起决定可达性
@unit ms';

comment on column msg_user.enabled is
'是否启用
adm_user.enabled 的副本；false 仍参与展开，只是不投递';

comment on column msg_user.creater_id is
'创建者ID
同步自 adm_user.creater_id';

comment on column msg_user.create_ms is
'创建时间戳
同步自 adm_user.create_ms
@unit ms';

comment on column msg_user.updater_id is
'更新者ID
同步自 adm_user.updater_id';

comment on column msg_user.update_ms is
'源行更新时间戳
取自 adm_user.update_ms，不是同步时间
@unit ms';

/*==============================================================*/
/* Index: msg_user_person_id                                    */
/*==============================================================*/
create  index msg_user_person_id on msg_user (
person_id
);

/*==============================================================*/
/* Table: msg_person                                            */
/*==============================================================*/
create table msg_person (
   id                   INT8                 not null,
   mobile               VARCHAR(50)          null,
   email                VARCHAR(50)          null,
   enabled              BOOL                 not null default true,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_PERSON primary key (id)
);

comment on table msg_person is
'人员投影（由同步写入，禁止在 msg 侧修改）
只为 user 缺地址时兜底；person 的 mobile/email 在 adm 侧不唯一，这里也不开 AK';

comment on column msg_person.id is
'ID
同 adm_person.id';

comment on column msg_person.mobile is
'手机号
user.mobile 为空时的短信兜底地址';

comment on column msg_person.email is
'邮箱
user.email 为空时的邮件兜底地址';

comment on column msg_person.enabled is
'是否启用
false 不阻断投递：地址照取，只做可观测';

comment on column msg_person.creater_id is
'创建者ID
同步自 adm_person.creater_id';

comment on column msg_person.create_ms is
'创建时间戳
同步自 adm_person.create_ms
@unit ms';

comment on column msg_person.updater_id is
'更新者ID
同步自 adm_person.updater_id';

comment on column msg_person.update_ms is
'源行更新时间戳
取自 adm_person.update_ms
@unit ms';

/*==============================================================*/
/* Table: msg_org                                               */
/*==============================================================*/
create table msg_org (
   id                   INT8                 not null,
   code                 VARCHAR(50)          null,
   tree_code            VARCHAR(30)          not null,
   name                 VARCHAR(50)          not null,
   full_name            VARCHAR(200)         null,
   enabled              BOOL                 not null default true,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_ORG primary key (id)
);

comment on table msg_org is
'组织投影（由同步写入，禁止在 msg 侧修改）
没有 parent_id：父子关系只由 tree_code 表达，每层 3 位、无分隔符（001 → 001002）
「含下级」= tree_code LIKE ''001002%''，所以 tree_code 上必须是 varchar_pattern_ops 索引';

comment on column msg_org.id is
'ID
同 adm_org.id';

comment on column msg_org.code is
'组织编码
adm 侧全局唯一；投影里不建 AK';

comment on column msg_org.tree_code is
'树编码
字典序即树的前序；总长 = 层数 × 3
@len 3..=30';

comment on column msg_org.name is
'名称
兄弟同名很常见，不做唯一';

comment on column msg_org.full_name is
'全称
可为空';

comment on column msg_org.enabled is
'是否启用
false 时自身及整棵子树都不投递（连带）';

comment on column msg_org.creater_id is
'创建者ID
同步自 adm_org.creater_id';

comment on column msg_org.create_ms is
'创建时间戳
同步自 adm_org.create_ms
@unit ms';

comment on column msg_org.updater_id is
'更新者ID
同步自 adm_org.updater_id';

comment on column msg_org.update_ms is
'源行更新时间戳
取自 adm_org.update_ms
@unit ms';

/*==============================================================*/
/* Index: msg_org_tree_code                                     */
/*==============================================================*/
create  index msg_org_tree_code on msg_org (
tree_code varchar_pattern_ops
);

/*==============================================================*/
/* Table: msg_role                                              */
/*==============================================================*/
create table msg_role (
   id                   INT8                 not null,
   group_id             INT8                 not null,
   name                 VARCHAR(50)          not null,
   enabled              BOOL                 not null default true,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_ROLE primary key (id)
);

comment on table msg_role is
'角色投影（由同步写入，禁止在 msg 侧修改）
adm 侧的 AK 是 (group_id, name)，投影只保留 id 做 PK，不建该 AK';

comment on column msg_role.id is
'ID
同 adm_role.id';

comment on column msg_role.group_id is
'角色分组ID
只在页面按分组展示时用，不参与投递';

comment on column msg_role.name is
'名称
同一分组内唯一，投影里不建 AK';

comment on column msg_role.enabled is
'是否启用
false 时不投递该角色';

comment on column msg_role.creater_id is
'创建者ID
同步自 adm_role.creater_id';

comment on column msg_role.create_ms is
'创建时间戳
同步自 adm_role.create_ms
@unit ms';

comment on column msg_role.updater_id is
'更新者ID
同步自 adm_role.updater_id';

comment on column msg_role.update_ms is
'源行更新时间戳
取自 adm_role.update_ms
@unit ms';

/*==============================================================*/
/* Index: msg_role_group_id                                     */
/*==============================================================*/
create  index msg_role_group_id on msg_role (
group_id
);

/*==============================================================*/
/* Table: msg_org_user                                          */
/*==============================================================*/
create table msg_org_user (
   id                   INT8                 not null,
   org_id               INT8                 not null,
   user_id              INT8                 not null,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_ORG_USER primary key (id)
);

comment on table msg_org_user is
'组织用户投影（由同步写入，禁止在 msg 侧修改）
多对多，没有「主组织」概念
调岗在 adm 侧是物理删行，靠 CDC 的 delete 事件同步删除';

comment on column msg_org_user.id is
'ID
同 adm_org_user.id';

comment on column msg_org_user.org_id is
'组织ID
关联 msg_org.id';

comment on column msg_org_user.user_id is
'用户ID
关联 msg_user.id';

comment on column msg_org_user.creater_id is
'创建者ID
同步自 adm_org_user.creater_id';

comment on column msg_org_user.create_ms is
'创建时间戳
同步自 adm_org_user.create_ms
@unit ms';

comment on column msg_org_user.updater_id is
'更新者ID
同步自 adm_org_user.updater_id';

comment on column msg_org_user.update_ms is
'源行更新时间戳
取自 adm_org_user.update_ms
@unit ms';

/*==============================================================*/
/* Index: msg_org_user_org_id                                   */
/*==============================================================*/
create  index msg_org_user_org_id on msg_org_user (
org_id
);

/*==============================================================*/
/* Index: msg_org_user_user_id                                  */
/*==============================================================*/
create  index msg_org_user_user_id on msg_org_user (
user_id
);

/*==============================================================*/
/* Table: msg_role_user                                         */
/*==============================================================*/
create table msg_role_user (
   id                   INT8                 not null,
   role_id              INT8                 not null,
   user_id              INT8                 not null,
   creater_id           INT8                 not null,
   create_ms            INT8                 not null,
   updater_id           INT8                 not null,
   update_ms            INT8                 not null,
   constraint PK_MSG_ROLE_USER primary key (id)
);

comment on table msg_role_user is
'角色用户投影（由同步写入，禁止在 msg 侧修改）
「按角色投递」展开的唯一依据
移出角色在 adm 侧是物理删行，靠 CDC 的 delete 事件同步删除';

comment on column msg_role_user.id is
'ID
同 adm_role_user.id';

comment on column msg_role_user.role_id is
'角色ID
关联 msg_role.id';

comment on column msg_role_user.user_id is
'用户ID
关联 msg_user.id';

comment on column msg_role_user.creater_id is
'创建者ID
同步自 adm_role_user.creater_id';

comment on column msg_role_user.create_ms is
'创建时间戳
同步自 adm_role_user.create_ms
@unit ms';

comment on column msg_role_user.updater_id is
'更新者ID
同步自 adm_role_user.updater_id';

comment on column msg_role_user.update_ms is
'源行更新时间戳
取自 adm_role_user.update_ms
@unit ms';

/*==============================================================*/
/* Index: msg_role_user_role_id                                 */
/*==============================================================*/
create  index msg_role_user_role_id on msg_role_user (
role_id
);

/*==============================================================*/
/* Index: msg_role_user_user_id                                 */
/*==============================================================*/
create  index msg_role_user_user_id on msg_role_user (
user_id
);
