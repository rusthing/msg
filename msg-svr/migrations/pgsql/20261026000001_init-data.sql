-- 初始化数据：消息中心跑起来所需的字典与入口配置
--
-- 与 create-table 分工：那边只建结构，本文件只插数据。四段的依赖只剩业务语义：
-- 默认目标类别要挂到 websocket 渠道上，渠道必须先插；投递队列只认主题
-- （`msg.delivery.{渠道编码}`），库里没有外键，顺序上不再依赖渠道行。
--
-- 每条记录都用个位数固定 ID：id-worker 生成的是雪花 ID（长整型大数），不会与这些
-- 小 ID 撞。sqlx 每个迁移只执行一次，这里写 `on conflict do nothing` 是为了兼容
-- 已经手工插过同名记录的库，不是给重复执行兜底。
-- 审计字段的 0 是 SYS_OPERATOR_USER_ID（系统操作员），时间戳取执行时刻。

/*==============================================================*/
/* 1. 普通消息类别                                              */
/*==============================================================*/

-- 消息必须挂在类别上：msg_message.category_id、msg_delivery.message_category_id
-- 都是非空外键，没有类别既注册不了消息也建不了投递记录。
-- 这里放一条「普通消息」兜底：业务方注册消息时没特别区分类别的都指向它。
insert into msg_message_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
) values (
   1,
   'normal',
   '普通消息',
   '默认类别：未特别区分类别的业务消息',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;

/*==============================================================*/
/* 2. websocket 渠道                                            */
/*==============================================================*/

-- 渠道是投递的出口：投递侧按 channel_code 找发送器（msg-svr/src/wkr/channel），
-- 投递主题与订阅方式则由 msg_delivery_queue 里配给它的那条记录决定。
-- 没有 websocket 这条渠道时，大屏握手直接返回 503：没有渠道配置，就没人往这个
-- 渠道投递，连上也收不到消息。
--
-- `options` 是握手校验用的配置：
--   {"token": "xxxx"} → 大屏连接时 ?token=xxxx 必须一致
--   {"token": ""}     → 不校验（联调用）
-- 换令牌只改这个字段，不用重启服务（每次握手现查渠道表）。
insert into msg_channel (
   id, code, name, options, remark, enabled, creater_id, create_ms, updater_id, update_ms
) values (
   2,
   'websocket',
   'websocket',
   '{"token":""}',
   'WebSocket长连接',
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;

/*==============================================================*/
/* 3. 默认目标类别                                              */
/*==============================================================*/

-- 目标类别是投递目标的字典：消息发给「谁」先落到类别上，再按类别找具体目标，
-- 或用空串 target_id 声明「该类别下的全部目标」。用户 / 角色 / 分组是平台级的
-- 固定口径，随库种好，发布方登记消息目标时按 code 查 ID，不用自己建。
--
-- 「大屏」（code = screen）不在这里：它是动态接入的自定义类别，由第一个用到它的
-- 发布方在启动时登记（dtw 就是这么做的），编码与名称在发布方配置里。
insert into msg_target_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
) values
(
   1, 'user', '用户', '按用户投递：目标ID是用户ID',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
),
(
   2, 'role', '角色', '按角色投递：目标ID是用户ID，角色下的用户都全部投递',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
),
(
   3, 'group', '分组', '按分组投递：目标ID是用户ID，分组内的用户都收',
   0, (extract(epoch from now()) * 1000)::int8,
   0, (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;

/*==============================================================*/
/* 4. websocket 投递队列                                        */
/*==============================================================*/

-- 一个渠道一条投递队列：订阅哪个 subject、要不要持久化，全看这张表。
-- 没配队列的渠道既不会被订阅，发布侧也发不出去（按渠道查不到队列就记失败）。
--
-- `code` 就是投递主题，固定为 `msg.delivery.{渠道编码}`，即 msg.delivery.websocket。
-- 这张表不存渠道 ID：渠道编码从主题末段解析，写入时校验渠道存在
-- （见 MsgDeliveryQueueSvc 的 before_add / before_modify），渠道被禁用后
-- 消息中心不再为它建订阅。
-- `persisted = false` 是 websocket 的硬要求：屏连在哪个实例上不确定，消息要让
-- 所有实例都收到、各自查本地连接表；落 JetStream 走队列组，反而会让「A 实例收
-- 任务、屏却连在 B 实例」的消息被别的实例吃掉。
insert into msg_delivery_queue (
   id, code, name, persisted, remark,
   creater_id, create_ms, updater_id, update_ms
)
values (
   1,
   'msg.delivery.websocket',
   'websocket 投递队列',
   false,
   'WebSocket长连接广播，所有实例都收',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;
