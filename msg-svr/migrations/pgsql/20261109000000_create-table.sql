/*==============================================================*/
/* DBMS name:      PostgreSQL 9.x                               */
/* Created on:     2026/10/18 12:58:42                          */
/*==============================================================*/


/*==============================================================*/
/* Table: msg_channel                                           */
/*==============================================================*/
create table msg_channel (
                             id                   INT8                 not null,
                             code                 VARCHAR(50)          not null,
                             name                 VARCHAR(50)          not null,
                             options              TEXT                 null,
                             payload_template     TEXT                 null,
                             remark               VARCHAR(50)          null,
                             enabled              BOOL                 not null default true,
                             creater_id           INT8                 not null,
                             create_ms            INT8                 not null,
                             updater_id           INT8                 not null,
                             update_ms            INT8                 not null,
                             constraint PK_MSG_CHANNEL primary key (id),
                             constraint AK_CODE_MSG_CHANNEL unique (code)
);

comment on table msg_channel is
'渠道
投递的出口，一个渠道一种投递方式
目前实现的是 websocket 长连接，新增渠道要同时实现发送器并配一条投递队列';

comment on column msg_channel.id is
'ID';

comment on column msg_channel.code is
'编码
@len 1..=50';

comment on column msg_channel.name is
'名称
@len 1..=50';

comment on column msg_channel.options is
'配置
@len 0..';

comment on column msg_channel.payload_template is
'载荷模板
渲染成这个渠道真正的报文正文，为空时用渠道代码里的默认结构（websocket 是 eventCode/title/content/ext/firstReceiveMs/businessTriggerMs）
变量只有服务端给的这几个：{{eventCode}} {{title}} {{content}} {{ext}} {{firstReceiveMs}} {{businessTriggerMs}}，语法同消息模板（{{变量:过滤器}}，字符串用 :json）
websocket 渠道要求渲染结果是合法 JSON，不是就告警并回退默认帧
@len 0..';

comment on column msg_channel.remark is
'备注
@len 0..=50';

comment on column msg_channel.enabled is
'启用';

comment on column msg_channel.creater_id is
'创建者ID';

comment on column msg_channel.create_ms is
'创建时间戳';

comment on column msg_channel.updater_id is
'更新者ID';

comment on column msg_channel.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_channel_PK                                        */
/*==============================================================*/
create unique index msg_channel_PK on msg_channel (
                                                   id
    );

/*==============================================================*/
/* Table: msg_dead_letter                                       */
/*==============================================================*/
create table msg_dead_letter (
                                 id                   INT8                 not null,
                                 subject              VARCHAR(150)         not null,
                                 reason               TEXT                 not null,
                                 payload              TEXT                 not null,
                                 delivered            INT8                 not null default 0,
                                 handled              BOOL                 not null default false,
                                 remark               VARCHAR(50)          null,
                                 simulated            BOOL                 not null default false,
                                 creater_id           INT8                 not null,
                                 create_ms            INT8                 not null,
                                 updater_id           INT8                 not null,
                                 update_ms            INT8                 not null,
                                 constraint PK_MSG_DEAD_LETTER primary key (id)
);

comment on table msg_dead_letter is
'死信消息
重投达到上限后被转入 dlq.{原subject} 的消息，供人工排查与重放';

comment on column msg_dead_letter.id is
'ID';

comment on column msg_dead_letter.subject is
'原消息主题
@len 1..=150';

comment on column msg_dead_letter.reason is
'死信原因
@len 1..';

comment on column msg_dead_letter.payload is
'消息内容
@len 1..';

comment on column msg_dead_letter.delivered is
'投递次数';

comment on column msg_dead_letter.handled is
'已处理';

comment on column msg_dead_letter.remark is
'备注
@len 0..=50';

comment on column msg_dead_letter.simulated is
'是否模拟：死信落库时按原始负载的 simulated=true 标记写入';

comment on column msg_dead_letter.creater_id is
'创建者ID';

comment on column msg_dead_letter.create_ms is
'创建时间戳';

comment on column msg_dead_letter.updater_id is
'更新者ID';

comment on column msg_dead_letter.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_dead_letter_PK                                    */
/*==============================================================*/
create unique index msg_dead_letter_PK on msg_dead_letter (
                                                           id
    );

/*==============================================================*/
/* Table: msg_delivery                                          */
/*==============================================================*/
create table msg_delivery (
                              id                   INT8                 not null,
                              event_code           VARCHAR(50)          not null,
                              message_id           INT8                 not null,
                              event_source_id      INT8                 not null,
                              message_category_id  INT8                 not null,
                              business_id          INT8                 not null,
                              business_trigger_ms  INT8                 not null,
                              receive_ms           INT8                 not null,
                              first_receive_ms     INT8                 null,
                              first_trigger_ms     INT8                 null,
                              deliver_status       INT2                 not null default 0,
                              labels               TEXT                 null,
                              annotations          TEXT                 null,
                              title                VARCHAR(150)         not null,
                              content              TEXT                 not null,
                              ext                  TEXT                 null,
                              remark               VARCHAR(50)          null,
                              simulated            BOOL                 not null default false,
                              creater_id           INT8                 not null,
                              create_ms            INT8                 not null,
                              updater_id           INT8                 not null,
                              update_ms            INT8                 not null,
                              constraint PK_MSG_DELIVERY primary key (id),
                              constraint AK_EVENT_CODE_AND_BUSINESS_ID_AND_BUSINESS_TRIGGER_MS_MSG_DELIVERY unique (event_code, business_id, business_trigger_ms)
);

comment on table msg_delivery is
'投递';

comment on column msg_delivery.id is
'ID';

comment on column msg_delivery.event_code is
'事件编码
在应用中定义，事件触发时传递出来
@len 1..=50';

comment on column msg_delivery.message_id is
'消息ID';

comment on column msg_delivery.event_source_id is
'事件来源ID';

comment on column msg_delivery.message_category_id is
'消息类别ID';

comment on column msg_delivery.business_id is
'业务ID
可以由业务触发时生成ID或者直接采用业务ID，与业务触发时间戳组合唯一约束，用于幂等去重，避免多次投递';

comment on column msg_delivery.business_trigger_ms is
'业务触发时间戳
与业务ID组合唯一约束，用于幂等去重，避免多次投递
是「这一次通知」的时间，每次通知都不同（报警的重复提醒靠它区分）；事件起点看 first_trigger_ms
取值跟着幂等键走：事件触发的通知用事件本身的时间（重投/重放不变），人为操作或定时提醒用发起时刻';

comment on column msg_delivery.receive_ms is
'接收时间戳
消息中心收到这条消息、开始落库处理的时刻（服务端时钟）
与 business_trigger_ms（发布方给的触发时间）之差就是发布方到消息中心的链路延迟';

comment on column msg_delivery.first_receive_ms is
'首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），由发布方透传下来，不是本服务打的点
与 first_trigger_ms 的区别是「谁的时钟」：那个是业务事实（事件发生了），这个是对时事实（我们的服务什么时候知道的）
发布方没接入或拿不到时为空；要算不受终端时钟偏差影响的端到端延迟，只能用这一列';

comment on column msg_delivery.first_trigger_ms is
'事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），同一条业务对象的多条投递记录同值
与 business_trigger_ms 分工不同：那个是「这一次通知」的时间并参与幂等，这个是「这件事什么时候开始发生」且不参与幂等
与渠道日志的 finish_ms 之差是「事件发生 → 送达」，但这个差值含发布方时钟偏差；发布方拿不到事件起点时为空';

comment on column msg_delivery.deliver_status is
'投递状态
@enum DeliverStatus default=Delivering
  0=Delivering      投递中
  1=PartialSuccess  投递部分成功
  2=AllSuccess      投递全部成功
  3=AllFailed       投递全部失败';

comment on column msg_delivery.labels is
'路由标签
一组结构化的 key-value，所有规则匹配、分组、路由、静默、维护窗口的过滤条件，都只针对 labels 做键值匹配或正则匹配，不碰任何自由文本
@len 0..';

comment on column msg_delivery.annotations is
'标注
放标题、详细描述、建议处理步骤这类可读文本，不参与任何匹配逻辑，只用于通知渲染和界面展示
@len 0..';

comment on column msg_delivery.title is
'标题
@len 1..=150';

comment on column msg_delivery.content is
'内容
@len 1..';

comment on column msg_delivery.ext is
'附加内容
模板渲染结果，格式不约定
@len 0..';

comment on column msg_delivery.remark is
'备注
@len 0..=50';

comment on column msg_delivery.simulated is
'是否模拟：发布方带 simulated=true 标记时为是';

comment on column msg_delivery.creater_id is
'创建者ID';

comment on column msg_delivery.create_ms is
'创建时间戳';

comment on column msg_delivery.updater_id is
'更新者ID';

comment on column msg_delivery.update_ms is
'更新时间戳';

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
/* Index: Relationship_16_FK                                    */
/*==============================================================*/
create  index Relationship_16_FK on msg_delivery (
                                                  event_source_id
    );

/*==============================================================*/
/* Index: Relationship_17_FK                                    */
/*==============================================================*/
create  index Relationship_17_FK on msg_delivery (
                                                  message_category_id
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
                                      creater_id           INT8                 not null,
                                      create_ms            INT8                 not null,
                                      updater_id           INT8                 not null,
                                      update_ms            INT8                 not null,
                                      constraint PK_MSG_DELIVERY_CHANNEL primary key (id),
                                      constraint AK_DELIVER_TARGET_ID_AND_CHANNEL_ID_MSG_DELIVERY_CHANNEL unique (deliver_target_id, channel_id)
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
@enum DeliverChannelStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败';

comment on column msg_delivery_channel.address is
'投递地址
@len 0..=80';

comment on column msg_delivery_channel.creater_id is
'创建者ID';

comment on column msg_delivery_channel.create_ms is
'创建时间戳';

comment on column msg_delivery_channel.updater_id is
'更新者ID';

comment on column msg_delivery_channel.update_ms is
'更新时间戳';

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
                                          first_trigger_ms     INT8                 null,
                                          first_receive_ms     INT8                 null,
                                          receive_ms           INT8                 not null,
                                          finish_ms            INT8                 null,
                                          business_trigger_ms  INT8                 not null,
                                          deliver_channel_status INT2                 not null default 0,
                                          address              VARCHAR(80)          null,
                                          detail               TEXT                 null,
                                          creater_id           INT8                 not null,
                                          create_ms            INT8                 not null,
                                          updater_id           INT8                 not null,
                                          update_ms            INT8                 not null,
                                          constraint PK_MSG_DELIVERY_CHANNEL_LOG primary key (id)
);

comment on table msg_delivery_channel_log is
'投递渠道日志';

comment on column msg_delivery_channel_log.id is
'ID';

comment on column msg_delivery_channel_log.delivery_channel_id is
'投递渠道ID';

comment on column msg_delivery_channel_log.first_trigger_ms is
'事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），取自投递任务带下来的 first_trigger_ms
同一条业务对象的多条记录同值；发布方没给时为空，与 finish_ms 之差是「事件发生 → 送达」，含发布方时钟偏差';

comment on column msg_delivery_channel_log.first_receive_ms is
'首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），从投递任务带下来
渠道日志可以脱离投递表单独算「链路首收 → 送达」的端到端延迟，且不受终端时钟偏差影响
发布方没给时为空';

comment on column msg_delivery_channel_log.receive_ms is
'接收时间戳
渠道消费到这条投递任务的时刻，也就是本次发送尝试的开始（服务端时钟）
与 finish_ms 之差是渠道内的发送耗时';

comment on column msg_delivery_channel_log.finish_ms is
'结束时间戳
本次发送尝试结束（成功或失败）的时刻（服务端时钟）
与 receive_ms 之差是渠道内的发送耗时；发送中崩溃时为空，渠道状态停在「投递中」';

comment on column msg_delivery_channel_log.business_trigger_ms is
'业务触发时间戳
本次投递对应的业务触发时间，取自投递任务带下来的 business_trigger_ms
与 msg_delivery.business_trigger_ms 同值，渠道日志可以脱离投递表单独算「通知到发送完成」的延迟
与 first_trigger_ms 的区别：这个是「这一次通知」的时间，每次通知都不同；那个是「事件首次发生」的时间';

comment on column msg_delivery_channel_log.deliver_channel_status is
'投递状态
@enum DeliverChannelStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败';

comment on column msg_delivery_channel_log.address is
'投递地址
@len 0..=80';

comment on column msg_delivery_channel_log.detail is
'投递详情
@len 0..';

comment on column msg_delivery_channel_log.creater_id is
'创建者ID';

comment on column msg_delivery_channel_log.create_ms is
'创建时间戳';

comment on column msg_delivery_channel_log.updater_id is
'更新者ID';

comment on column msg_delivery_channel_log.update_ms is
'更新时间戳';

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
/* Table: msg_delivery_queue                                    */
/*==============================================================*/
create table msg_delivery_queue (
                                    id                   INT8                 not null,
                                    code                 VARCHAR(50)          not null,
                                    name                 VARCHAR(50)          not null,
                                    persisted            BOOL                 not null default false,
                                    remark               VARCHAR(50)          null,
                                    creater_id           INT8                 not null,
                                    create_ms            INT8                 not null,
                                    updater_id           INT8                 not null,
                                    update_ms            INT8                 not null,
                                    constraint PK_MSG_DELIVERY_QUEUE primary key (id),
                                    constraint AK_CODE_MSG_DELIVERY_QUEUE unique (code),
                                    constraint AK_NAME_MSG_DELIVERY_QUEUE unique (name)
);

comment on table msg_delivery_queue is
'投递队列
按渠道配置，消息中心据此订阅投递主题并决定是否持久化';

comment on column msg_delivery_queue.id is
'ID';

comment on column msg_delivery_queue.code is
'编码
投递主题，固定为 msg.delivery.{渠道编码}，末段即 msg_channel.code
@len 1..=50';

comment on column msg_delivery_queue.name is
'名称
@len 1..=50';

comment on column msg_delivery_queue.persisted is
'是否持久化
持久化走 JetStream（落盘、重投、队列组负载均衡）；不持久化走 Core NATS 广播';

comment on column msg_delivery_queue.remark is
'备注
@len 0..=50';

comment on column msg_delivery_queue.creater_id is
'创建者ID';

comment on column msg_delivery_queue.create_ms is
'创建时间戳';

comment on column msg_delivery_queue.updater_id is
'更新者ID';

comment on column msg_delivery_queue.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_delivery_queue_PK                                 */
/*==============================================================*/
create unique index msg_delivery_queue_PK on msg_delivery_queue (
                                                                 id
    );

/*==============================================================*/
/* Table: msg_delivery_target                                   */
/*==============================================================*/
create table msg_delivery_target (
                                     id                   INT8                 not null,
                                     delivery_id          INT8                 not null,
                                     user_id              INT8                 null,
                                     target_category_id   INT8                 not null,
                                     target_id            VARCHAR(100)         not null,
                                     origin               VARCHAR(200)         null,
                                     deliver_target_status INT2                 not null default 0,
                                     creater_id           INT8                 not null,
                                     create_ms            INT8                 not null,
                                     updater_id           INT8                 not null,
                                     update_ms            INT8                 not null,
                                     constraint PK_MSG_DELIVERY_TARGET primary key (id),
                                     constraint AK_DELIVERY_ID_AND_USER_ID_MSG_DELIVERY_TARGET unique (delivery_id, user_id),
                                     constraint AK_DELIVERY_ID_AND_TARGET_CATEGORY_ID_AND_TARGET_ID_MSG_DELIVERY_TARGET unique (delivery_id, target_category_id, target_id)
);

comment on table msg_delivery_target is
'投递目标';

comment on column msg_delivery_target.id is
'ID';

comment on column msg_delivery_target.user_id is
'展开后的用户ID
大屏 / 定位卡目标为空';

comment on column msg_delivery_target.origin is
'命中的原始目标
user:2,role:2,org:1001；多来源英文逗号分隔';

comment on column msg_delivery_target.delivery_id is
'投递ID';

comment on column msg_delivery_target.target_category_id is
'目标类别ID';

comment on column msg_delivery_target.target_id is
'目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）
@len 0..=100';

comment on column msg_delivery_target.deliver_target_status is
'投递状态
@enum DeliverTargetStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败
  3=Read        目标已读';

comment on column msg_delivery_target.creater_id is
'创建者ID';

comment on column msg_delivery_target.create_ms is
'创建时间戳';

comment on column msg_delivery_target.updater_id is
'更新者ID';

comment on column msg_delivery_target.update_ms is
'更新时间戳';

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

/*==============================================================*/
/* Index: msg_delivery_target_AK2                               */
/*==============================================================*/
create unique index msg_delivery_target_AK2 on msg_delivery_target (
delivery_id,
user_id
);
/* Table: msg_event_source                                      */
/*==============================================================*/
create table msg_event_source (
                                  id                   INT8                 not null,
                                  code                 VARCHAR(50)          not null,
                                  name                 VARCHAR(50)          not null,
                                  remark               VARCHAR(50)          null,
                                  creater_id           INT8                 not null,
                                  create_ms            INT8                 not null,
                                  updater_id           INT8                 not null,
                                  update_ms            INT8                 not null,
                                  constraint PK_MSG_EVENT_SOURCE primary key (id),
                                  constraint AK_CODE_MSG_EVENT_SOURCE unique (code),
                                  constraint AK_NAME_MSG_EVENT_SOURCE unique (name)
);

comment on table msg_event_source is
'事件来源
由客户端启动时自动注册上来';

comment on column msg_event_source.id is
'ID';

comment on column msg_event_source.code is
'编码
@len 1..=50';

comment on column msg_event_source.name is
'名称
@len 1..=50';

comment on column msg_event_source.remark is
'备注
@len 0..=50';

comment on column msg_event_source.creater_id is
'创建者ID';

comment on column msg_event_source.create_ms is
'创建时间戳';

comment on column msg_event_source.updater_id is
'更新者ID';

comment on column msg_event_source.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_event_source_PK                                   */
/*==============================================================*/
create unique index msg_event_source_PK on msg_event_source (
                                                             id
    );

/*==============================================================*/
/* Table: msg_message                                           */
/*==============================================================*/
create table msg_message (
                             id                   INT8                 not null,
                             event_source_id      INT8                 not null,
                             category_id          INT8                 not null,
                             name                 VARCHAR(50)          not null,
                             event_code           VARCHAR(50)          not null,
                             persisted            BOOL                 not null default true,
                             title_template       VARCHAR(150)         null,
                             content_template     TEXT                 null,
                             ext_template         TEXT                 null,
                             remark               VARCHAR(50)          null,
                             enabled              BOOL                 not null default true,
                             sim_enabled          BOOL                 not null default true,
                             creater_id           INT8                 not null,
                             create_ms            INT8                 not null,
                             updater_id           INT8                 not null,
                             update_ms            INT8                 not null,
                             constraint PK_MSG_MESSAGE primary key (id),
                             constraint AK_NAME_MSG_MESSAGE unique (name),
                             constraint AK_EVENT_CODE_MSG_MESSAGE unique (event_code)
);

comment on table msg_message is
'消息
由客户端启动时自动注册事件的基本信息上来，用户可以配置消息的其它信息，不能修改事件的基本信息更不能删除';

comment on column msg_message.id is
'ID';

comment on column msg_message.event_source_id is
'事件来源ID';

comment on column msg_message.category_id is
'消息类别ID';

comment on column msg_message.name is
'名称
@len 1..=50';

comment on column msg_message.event_code is
'事件编码
在应用中定义，事件触发时传递出来
@len 1..=50';

comment on column msg_message.persisted is
'是否持久化
如果不持久化，投递将不会保存到数据库';

comment on column msg_message.title_template is
'标题模板
为null的话用发布方给的title作为标题
@len 0..=150';

comment on column msg_message.content_template is
'内容模板
为null的话用发布方给的content作为内容
@len 0..';

comment on column msg_message.ext_template is
'附加内容模板
渲染结果原样下发，格式不约定；为null时用发布方给的附加内容
@len 0..';

comment on column msg_message.remark is
'备注
@len 0..=50';

comment on column msg_message.enabled is
'启用';

comment on column msg_message.sim_enabled is
'是否允许模拟通知投递
默认 true；关闭时带 simulated=true 标记的通知会被丢弃';

comment on column msg_message.creater_id is
'创建者ID';

comment on column msg_message.create_ms is
'创建时间戳';

comment on column msg_message.updater_id is
'更新者ID';

comment on column msg_message.update_ms is
'更新时间戳';

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
/* Index: Relationship_13_FK                                    */
/*==============================================================*/
create  index Relationship_13_FK on msg_message (
                                                 event_source_id
    );

/*==============================================================*/
/* Table: msg_message_category                                  */
/*==============================================================*/
create table msg_message_category (
                                      id                   INT8                 not null,
                                      code                 VARCHAR(50)          not null,
                                      name                 VARCHAR(50)          not null,
                                      remark               VARCHAR(50)          null,
                                      creater_id           INT8                 not null,
                                      create_ms            INT8                 not null,
                                      updater_id           INT8                 not null,
                                      update_ms            INT8                 not null,
                                      constraint PK_MSG_MESSAGE_CATEGORY primary key (id),
                                      constraint AK_CODE_MSG_MESSAGE_CATEGORY unique (code),
                                      constraint AK_NAME_MSG_MESSAGE_CATEGORY unique (name)
);

comment on table msg_message_category is
'消息类别
由客户端启动时自动注册上来';

comment on column msg_message_category.id is
'ID';

comment on column msg_message_category.code is
'编码
@len 1..=50';

comment on column msg_message_category.name is
'名称
@len 1..=50';

comment on column msg_message_category.remark is
'备注
@len 0..=50';

comment on column msg_message_category.creater_id is
'创建者ID';

comment on column msg_message_category.create_ms is
'创建时间戳';

comment on column msg_message_category.updater_id is
'更新者ID';

comment on column msg_message_category.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_message_category_PK                               */
/*==============================================================*/
create unique index msg_message_category_PK on msg_message_category (
                                                                     id
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
                                   creater_id           INT8                 not null,
                                   create_ms            INT8                 not null,
                                   updater_id           INT8                 not null,
                                   update_ms            INT8                 not null,
                                   constraint PK_MSG_MESSAGE_QUEUE primary key (id),
                                   constraint AK_CODE_MSG_MESSAGE_QUEUE unique (code),
                                   constraint AK_NAME_MSG_MESSAGE_QUEUE unique (name)
);

comment on table msg_message_queue is
'消息队列
发布方注册自己的消息主题，消息中心据此订阅并决定是否持久化';

comment on column msg_message_queue.id is
'ID';

comment on column msg_message_queue.code is
'编码
消息主题，订阅与发布都用它
@len 1..=50';

comment on column msg_message_queue.name is
'名称
@len 1..=50';

comment on column msg_message_queue.persisted is
'是否持久化
持久化走 JetStream（发布方等 ACK，服务重启后补消费）；不持久化走 Core NATS，发出即不管送达';

comment on column msg_message_queue.remark is
'备注
@len 0..=50';

comment on column msg_message_queue.creater_id is
'创建者ID';

comment on column msg_message_queue.create_ms is
'创建时间戳';

comment on column msg_message_queue.updater_id is
'更新者ID';

comment on column msg_message_queue.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_message_queue_PK                                  */
/*==============================================================*/
create unique index msg_message_queue_PK on msg_message_queue (
                                                               id
    );

/*==============================================================*/
/* Table: msg_message_target                                    */
/*==============================================================*/
create table msg_message_target (
                                    id                   INT8                 not null,
                                    message_id           INT8                 not null,
                                    target_category_id   INT8                 not null,
                                    target_id            VARCHAR(100)         not null,
                                    creater_id           INT8                 not null,
                                    create_ms            INT8                 not null,
                                    updater_id           INT8                 not null,
                                    update_ms            INT8                 not null,
                                    constraint PK_MSG_MESSAGE_TARGET primary key (id),
                                    constraint AK_MESSAGE_ID_AND_TARGET_CATEGORY_ID_AND_TARGET_ID_MSG_MESSAGE_TARGET unique (message_id, target_category_id, target_id)
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
'目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）
@len 0..=100';

comment on column msg_message_target.creater_id is
'创建者ID';

comment on column msg_message_target.create_ms is
'创建时间戳';

comment on column msg_message_target.updater_id is
'更新者ID';

comment on column msg_message_target.update_ms is
'更新时间戳';

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
                                     creater_id           INT8                 not null,
                                     create_ms            INT8                 not null,
                                     updater_id           INT8                 not null,
                                     update_ms            INT8                 not null,
                                     constraint PK_MSG_TARGET_CATEGORY primary key (id),
                                     constraint AK_CODE_MSG_TARGET_CATEGORY unique (code),
                                     constraint AK_NAME_MSG_TARGET_CATEGORY unique (name)
);

comment on table msg_target_category is
'目标类别
有用户/角色/分组或自定义如大屏';

comment on column msg_target_category.id is
'ID';

comment on column msg_target_category.code is
'编码
@len 1..=50';

comment on column msg_target_category.name is
'名称
@len 1..=50';

comment on column msg_target_category.remark is
'备注
@len 0..=50';

comment on column msg_target_category.creater_id is
'创建者ID';

comment on column msg_target_category.create_ms is
'创建时间戳';

comment on column msg_target_category.updater_id is
'更新者ID';

comment on column msg_target_category.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_target_category_PK                                */
/*==============================================================*/
create unique index msg_target_category_PK on msg_target_category (
                                                                   id
    );

/*==============================================================*/
/* Table: msg_target_category_channel                           */
/*==============================================================*/
create table msg_target_category_channel (
                                             id                   INT8                 not null,
                                             target_category_id   INT8                 not null,
                                             channel_id           INT8                 not null,
                                             creater_id           INT8                 not null,
                                             create_ms            INT8                 not null,
                                             updater_id           INT8                 not null,
                                             update_ms            INT8                 not null,
                                             constraint PK_MSG_TARGET_CATEGORY_CHANNEL primary key (id),
                                             constraint AK_CHANNEL_ID_AND_TARGET_CATEGORY_ID_MSG_TARGET_CATEGORY_CHANNEL unique (channel_id, target_category_id)
);

comment on table msg_target_category_channel is
'目标类别渠道（默认渠道集）
只用于新建消息目标时给 UI 预填，不参与投递
投递只认 msg_message_target_channel';

comment on column msg_target_category_channel.id is
'ID';

comment on column msg_target_category_channel.target_category_id is
'目标类别ID';

comment on column msg_target_category_channel.channel_id is
'渠道ID';

comment on column msg_target_category_channel.creater_id is
'创建者ID';

comment on column msg_target_category_channel.create_ms is
'创建时间戳';

comment on column msg_target_category_channel.updater_id is
'更新者ID';

comment on column msg_target_category_channel.update_ms is
'更新时间戳';

/*==============================================================*/
/* Index: msg_target_category_channel_PK                        */
/*==============================================================*/
create unique index msg_target_category_channel_PK on msg_target_category_channel (
                                                                                   id
    );

/*==============================================================*/
/* Index: Relationship_14_FK                                    */
/*==============================================================*/
create  index Relationship_14_FK on msg_target_category_channel (
                                                                 target_category_id
    );

/*==============================================================*/
/* Index: Relationship_15_FK                                    */
/*==============================================================*/
create  index Relationship_15_FK on msg_target_category_channel (
                                                                 channel_id
    );

alter table msg_delivery
    add constraint fk_event_source_id__from__msg_event_source foreign key (event_source_id)
        references msg_event_source (id)
        on delete restrict on update restrict;

alter table msg_delivery
    add constraint fk_message_category_id__from__msg_message_category foreign key (message_category_id)
        references msg_message_category (id)
        on delete restrict on update restrict;

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
    add constraint fk_event_source_id__from__msg_event_source foreign key (event_source_id)
        references msg_event_source (id)
        on delete restrict on update restrict;

alter table msg_message_target
    add constraint fk_target_category_id__from__msg_target_category foreign key (target_category_id)
        references msg_target_category (id)
        on delete restrict on update restrict;

alter table msg_message_target
    add constraint fk_message_id__from__msg_message foreign key (message_id)
        references msg_message (id)
        on delete restrict on update restrict;

alter table msg_target_category_channel
    add constraint fk_target_category_id__from__msg_target_category foreign key (target_category_id)
        references msg_target_category (id)
        on delete restrict on update restrict;

alter table msg_target_category_channel
    add constraint fk_channel_id__from__msg_channel foreign key (channel_id)
        references msg_channel (id)
        on delete restrict on update restrict;

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