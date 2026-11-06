-- msg 数据库迁移基线：init-create-table（最终态建表）
--
-- 本文件由 A 库（重放历史全部迁移 + 生产补种）导出，用于替代历史迁移。
-- 一旦发布并写入 _sqlx_migrations，禁止再修改本文件；后续变更必须新增迁移。
-- 生成来源：A 库 pg_dump --schema-only（逐字节还原列序、约束名、列注释）

CREATE TABLE public.msg_channel (
    id bigint NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    options text,
    payload_template text,
    remark character varying(50),
    enabled boolean DEFAULT true NOT NULL,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_channel IS '渠道
投递的出口，一个渠道一种投递方式
目前实现的是 websocket 长连接，新增渠道要同时实现发送器并配一条投递队列';

COMMENT ON COLUMN public.msg_channel.id IS 'ID';

COMMENT ON COLUMN public.msg_channel.code IS '编码
@len 1..=50';

COMMENT ON COLUMN public.msg_channel.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_channel.options IS '配置
@len 0..';

COMMENT ON COLUMN public.msg_channel.payload_template IS '载荷模板
渲染成这个渠道真正的报文正文，为空时用渠道代码里的默认结构（websocket 是 eventCode/title/content/ext/firstReceiveMs/businessTriggerMs）
变量只有服务端给的这几个：{{eventCode}} {{title}} {{content}} {{ext}} {{firstReceiveMs}} {{businessTriggerMs}}，语法同消息模板（{{变量:过滤器}}，字符串用 :json）
websocket 渠道要求渲染结果是合法 JSON，不是就告警并回退默认帧
@len 0..';

COMMENT ON COLUMN public.msg_channel.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_channel.enabled IS '启用';

COMMENT ON COLUMN public.msg_channel.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_channel.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_channel.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_channel.update_ms IS '更新时间戳';

CREATE TABLE public.msg_dead_letter (
    id bigint NOT NULL,
    subject character varying(150) NOT NULL,
    reason text NOT NULL,
    payload text NOT NULL,
    delivered bigint DEFAULT 0 NOT NULL,
    handled boolean DEFAULT false NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_dead_letter IS '死信消息
重投达到上限后被转入 dlq.{原subject} 的消息，供人工排查与重放';

COMMENT ON COLUMN public.msg_dead_letter.id IS 'ID';

COMMENT ON COLUMN public.msg_dead_letter.subject IS '原消息主题
@len 1..=150';

COMMENT ON COLUMN public.msg_dead_letter.reason IS '死信原因
@len 1..';

COMMENT ON COLUMN public.msg_dead_letter.payload IS '消息内容
@len 1..';

COMMENT ON COLUMN public.msg_dead_letter.delivered IS '投递次数';

COMMENT ON COLUMN public.msg_dead_letter.handled IS '已处理';

COMMENT ON COLUMN public.msg_dead_letter.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_dead_letter.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_dead_letter.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_dead_letter.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_dead_letter.update_ms IS '更新时间戳';

CREATE TABLE public.msg_delivery (
    id bigint NOT NULL,
    event_code character varying(50) NOT NULL,
    message_id bigint NOT NULL,
    event_source_id bigint NOT NULL,
    message_category_id bigint NOT NULL,
    business_id bigint NOT NULL,
    business_trigger_ms bigint NOT NULL,
    receive_ms bigint NOT NULL,
    first_receive_ms bigint,
    first_trigger_ms bigint,
    deliver_status smallint DEFAULT 0 NOT NULL,
    labels text,
    annotations text,
    title character varying(150) NOT NULL,
    content text NOT NULL,
    ext text,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_delivery IS '投递';

COMMENT ON COLUMN public.msg_delivery.id IS 'ID';

COMMENT ON COLUMN public.msg_delivery.event_code IS '事件编码
在应用中定义，事件触发时传递出来
@len 1..=50';

COMMENT ON COLUMN public.msg_delivery.message_id IS '消息ID';

COMMENT ON COLUMN public.msg_delivery.event_source_id IS '事件来源ID';

COMMENT ON COLUMN public.msg_delivery.message_category_id IS '消息类别ID';

COMMENT ON COLUMN public.msg_delivery.business_id IS '业务ID
可以由业务触发时生成ID或者直接采用业务ID，与业务触发时间戳组合唯一约束，用于幂等去重，避免多次投递';

COMMENT ON COLUMN public.msg_delivery.business_trigger_ms IS '业务触发时间戳
与业务ID组合唯一约束，用于幂等去重，避免多次投递
是「这一次通知」的时间，每次通知都不同（报警的重复提醒靠它区分）；事件起点看 first_trigger_ms
取值跟着幂等键走：事件触发的通知用事件本身的时间（重投/重放不变），人为操作或定时提醒用发起时刻';

COMMENT ON COLUMN public.msg_delivery.receive_ms IS '接收时间戳
消息中心收到这条消息、开始落库处理的时刻（服务端时钟）
与 business_trigger_ms（发布方给的触发时间）之差就是发布方到消息中心的链路延迟';

COMMENT ON COLUMN public.msg_delivery.first_receive_ms IS '首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），由发布方透传下来，不是本服务打的点
与 first_trigger_ms 的区别是「谁的时钟」：那个是业务事实（事件发生了），这个是对时事实（我们的服务什么时候知道的）
发布方没接入或拿不到时为空；要算不受终端时钟偏差影响的端到端延迟，只能用这一列';

COMMENT ON COLUMN public.msg_delivery.first_trigger_ms IS '事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），同一条业务对象的多条投递记录同值
与 business_trigger_ms 分工不同：那个是「这一次通知」的时间并参与幂等，这个是「这件事什么时候开始发生」且不参与幂等
与渠道日志的 finish_ms 之差是「事件发生 → 送达」，但这个差值含发布方时钟偏差；发布方拿不到事件起点时为空';

COMMENT ON COLUMN public.msg_delivery.deliver_status IS '投递状态
@enum DeliverStatus default=Delivering
  0=Delivering      投递中
  1=PartialSuccess  投递部分成功
  2=AllSuccess      投递全部成功
  3=AllFailed       投递全部失败';

COMMENT ON COLUMN public.msg_delivery.labels IS '路由标签
一组结构化的 key-value，所有规则匹配、分组、路由、静默、维护窗口的过滤条件，都只针对 labels 做键值匹配或正则匹配，不碰任何自由文本
@len 0..';

COMMENT ON COLUMN public.msg_delivery.annotations IS '标注
放标题、详细描述、建议处理步骤这类可读文本，不参与任何匹配逻辑，只用于通知渲染和界面展示
@len 0..';

COMMENT ON COLUMN public.msg_delivery.title IS '标题
@len 1..=150';

COMMENT ON COLUMN public.msg_delivery.content IS '内容
@len 1..';

COMMENT ON COLUMN public.msg_delivery.ext IS '附加内容
模板渲染结果，格式不约定
@len 0..';

COMMENT ON COLUMN public.msg_delivery.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_delivery.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_delivery.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_delivery.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_delivery.update_ms IS '更新时间戳';

CREATE TABLE public.msg_delivery_channel (
    id bigint NOT NULL,
    deliver_target_id bigint NOT NULL,
    channel_id bigint NOT NULL,
    deliver_channel_status smallint DEFAULT 0 NOT NULL,
    address character varying(80),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_delivery_channel IS '投递渠道';

COMMENT ON COLUMN public.msg_delivery_channel.id IS 'ID';

COMMENT ON COLUMN public.msg_delivery_channel.deliver_target_id IS '投递目标ID';

COMMENT ON COLUMN public.msg_delivery_channel.channel_id IS '渠道ID';

COMMENT ON COLUMN public.msg_delivery_channel.deliver_channel_status IS '投递状态
@enum DeliverChannelStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败';

COMMENT ON COLUMN public.msg_delivery_channel.address IS '投递地址
@len 0..=80';

COMMENT ON COLUMN public.msg_delivery_channel.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_delivery_channel.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_delivery_channel.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_delivery_channel.update_ms IS '更新时间戳';

CREATE TABLE public.msg_delivery_channel_log (
    id bigint NOT NULL,
    delivery_channel_id bigint NOT NULL,
    first_trigger_ms bigint,
    first_receive_ms bigint,
    receive_ms bigint NOT NULL,
    finish_ms bigint,
    business_trigger_ms bigint NOT NULL,
    deliver_channel_status smallint DEFAULT 0 NOT NULL,
    address character varying(80),
    detail text,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_delivery_channel_log IS '投递渠道日志';

COMMENT ON COLUMN public.msg_delivery_channel_log.id IS 'ID';

COMMENT ON COLUMN public.msg_delivery_channel_log.delivery_channel_id IS '投递渠道ID';

COMMENT ON COLUMN public.msg_delivery_channel_log.first_trigger_ms IS '事件首次触发时间戳
发布方给的事件起点（业务/发布方时钟），取自投递任务带下来的 first_trigger_ms
同一条业务对象的多条记录同值；发布方没给时为空，与 finish_ms 之差是「事件发生 → 送达」，含发布方时钟偏差';

COMMENT ON COLUMN public.msg_delivery_channel_log.first_receive_ms IS '首次接收时间戳
链路第一跳收到这个原始事件的时刻（服务端时钟），从投递任务带下来
渠道日志可以脱离投递表单独算「链路首收 → 送达」的端到端延迟，且不受终端时钟偏差影响
发布方没给时为空';

COMMENT ON COLUMN public.msg_delivery_channel_log.receive_ms IS '接收时间戳
渠道消费到这条投递任务的时刻，也就是本次发送尝试的开始（服务端时钟）
与 finish_ms 之差是渠道内的发送耗时';

COMMENT ON COLUMN public.msg_delivery_channel_log.finish_ms IS '结束时间戳
本次发送尝试结束（成功或失败）的时刻（服务端时钟）
与 receive_ms 之差是渠道内的发送耗时；发送中崩溃时为空，渠道状态停在「投递中」';

COMMENT ON COLUMN public.msg_delivery_channel_log.business_trigger_ms IS '业务触发时间戳
本次投递对应的业务触发时间，取自投递任务带下来的 business_trigger_ms
与 msg_delivery.business_trigger_ms 同值，渠道日志可以脱离投递表单独算「通知到发送完成」的延迟
与 first_trigger_ms 的区别：这个是「这一次通知」的时间，每次通知都不同；那个是「事件首次发生」的时间';

COMMENT ON COLUMN public.msg_delivery_channel_log.deliver_channel_status IS '投递状态
@enum DeliverChannelStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败';

COMMENT ON COLUMN public.msg_delivery_channel_log.address IS '投递地址
@len 0..=80';

COMMENT ON COLUMN public.msg_delivery_channel_log.detail IS '投递详情
@len 0..';

COMMENT ON COLUMN public.msg_delivery_channel_log.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_delivery_channel_log.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_delivery_channel_log.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_delivery_channel_log.update_ms IS '更新时间戳';

CREATE TABLE public.msg_delivery_queue (
    id bigint NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    persisted boolean DEFAULT false NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_delivery_queue IS '投递队列
按渠道配置，消息中心据此订阅投递主题并决定是否持久化';

COMMENT ON COLUMN public.msg_delivery_queue.id IS 'ID';

COMMENT ON COLUMN public.msg_delivery_queue.code IS '编码
投递主题，固定为 msg.delivery.{渠道编码}，末段即 msg_channel.code
@len 1..=50';

COMMENT ON COLUMN public.msg_delivery_queue.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_delivery_queue.persisted IS '是否持久化
持久化走 JetStream（落盘、重投、队列组负载均衡）；不持久化走 Core NATS 广播';

COMMENT ON COLUMN public.msg_delivery_queue.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_delivery_queue.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_delivery_queue.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_delivery_queue.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_delivery_queue.update_ms IS '更新时间戳';

CREATE TABLE public.msg_delivery_target (
    id bigint NOT NULL,
    delivery_id bigint NOT NULL,
    target_category_id bigint NOT NULL,
    target_id character varying(100) NOT NULL,
    deliver_target_status smallint DEFAULT 0 NOT NULL,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_delivery_target IS '投递目标';

COMMENT ON COLUMN public.msg_delivery_target.id IS 'ID';

COMMENT ON COLUMN public.msg_delivery_target.delivery_id IS '投递ID';

COMMENT ON COLUMN public.msg_delivery_target.target_category_id IS '目标类别ID';

COMMENT ON COLUMN public.msg_delivery_target.target_id IS '目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）
@len 0..=100';

COMMENT ON COLUMN public.msg_delivery_target.deliver_target_status IS '投递状态
@enum DeliverTargetStatus default=Delivering
  0=Delivering  投递中
  1=Success     投递成功
  2=Failed      投递失败
  3=Read        目标已读';

COMMENT ON COLUMN public.msg_delivery_target.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_delivery_target.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_delivery_target.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_delivery_target.update_ms IS '更新时间戳';

CREATE TABLE public.msg_event_source (
    id bigint NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_event_source IS '事件来源
由客户端启动时自动注册上来';

COMMENT ON COLUMN public.msg_event_source.id IS 'ID';

COMMENT ON COLUMN public.msg_event_source.code IS '编码
@len 1..=50';

COMMENT ON COLUMN public.msg_event_source.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_event_source.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_event_source.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_event_source.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_event_source.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_event_source.update_ms IS '更新时间戳';

CREATE TABLE public.msg_message (
    id bigint NOT NULL,
    event_source_id bigint NOT NULL,
    category_id bigint NOT NULL,
    name character varying(50) NOT NULL,
    event_code character varying(50) NOT NULL,
    persisted boolean DEFAULT true NOT NULL,
    title_template character varying(150),
    content_template text,
    ext_template text,
    remark character varying(50),
    enabled boolean DEFAULT true NOT NULL,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL,
    sim_enabled boolean DEFAULT false NOT NULL
);

COMMENT ON TABLE public.msg_message IS '消息
由客户端启动时自动注册事件的基本信息上来，用户可以配置消息的其它信息，不能修改事件的基本信息更不能删除';

COMMENT ON COLUMN public.msg_message.id IS 'ID';

COMMENT ON COLUMN public.msg_message.event_source_id IS '事件来源ID';

COMMENT ON COLUMN public.msg_message.category_id IS '消息类别ID';

COMMENT ON COLUMN public.msg_message.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_message.event_code IS '事件编码
在应用中定义，事件触发时传递出来
@len 1..=50';

COMMENT ON COLUMN public.msg_message.persisted IS '是否持久化
如果不持久化，投递将不会保存到数据库';

COMMENT ON COLUMN public.msg_message.title_template IS '标题模板
为null的话用发布方给的title作为标题
@len 0..=150';

COMMENT ON COLUMN public.msg_message.content_template IS '内容模板
为null的话用发布方给的content作为内容
@len 0..';

COMMENT ON COLUMN public.msg_message.ext_template IS '附加内容模板
渲染结果原样下发，格式不约定；为null时用发布方给的附加内容
@len 0..';

COMMENT ON COLUMN public.msg_message.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_message.enabled IS '启用';

COMMENT ON COLUMN public.msg_message.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_message.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_message.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_message.update_ms IS '更新时间戳';

COMMENT ON COLUMN public.msg_message.sim_enabled IS '是否允许模拟通知投递
默认 false；关闭时带 simulated=true / source=sim 标记的通知会被丢弃';

CREATE TABLE public.msg_message_category (
    id bigint CONSTRAINT msg_message_category_id_not_null1 NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_message_category IS '消息类别
由客户端启动时自动注册上来';

COMMENT ON COLUMN public.msg_message_category.id IS 'ID';

COMMENT ON COLUMN public.msg_message_category.code IS '编码
@len 1..=50';

COMMENT ON COLUMN public.msg_message_category.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_message_category.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_message_category.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_message_category.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_message_category.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_message_category.update_ms IS '更新时间戳';

CREATE TABLE public.msg_message_queue (
    id bigint NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    persisted boolean DEFAULT true NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_message_queue IS '消息队列
发布方注册自己的消息主题，消息中心据此订阅并决定是否持久化';

COMMENT ON COLUMN public.msg_message_queue.id IS 'ID';

COMMENT ON COLUMN public.msg_message_queue.code IS '编码
消息主题，订阅与发布都用它
@len 1..=50';

COMMENT ON COLUMN public.msg_message_queue.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_message_queue.persisted IS '是否持久化
持久化走 JetStream（发布方等 ACK，服务重启后补消费）；不持久化走 Core NATS，发出即不管送达';

COMMENT ON COLUMN public.msg_message_queue.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_message_queue.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_message_queue.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_message_queue.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_message_queue.update_ms IS '更新时间戳';

CREATE TABLE public.msg_message_target (
    id bigint NOT NULL,
    message_id bigint NOT NULL,
    target_category_id bigint NOT NULL,
    target_id character varying(100) NOT NULL,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_message_target IS '消息目标';

COMMENT ON COLUMN public.msg_message_target.id IS 'ID';

COMMENT ON COLUMN public.msg_message_target.message_id IS '消息ID';

COMMENT ON COLUMN public.msg_message_target.target_category_id IS '目标类别ID';

COMMENT ON COLUMN public.msg_message_target.target_id IS '目标标识
空串表示该类别下的全部目标（大屏动态接入，不逐个登记）
@len 0..=100';

COMMENT ON COLUMN public.msg_message_target.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_message_target.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_message_target.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_message_target.update_ms IS '更新时间戳';

CREATE TABLE public.msg_target_category (
    id bigint NOT NULL,
    code character varying(50) NOT NULL,
    name character varying(50) NOT NULL,
    remark character varying(50),
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_target_category IS '目标类别
有用户/角色/分组或自定义如大屏';

COMMENT ON COLUMN public.msg_target_category.id IS 'ID';

COMMENT ON COLUMN public.msg_target_category.code IS '编码
@len 1..=50';

COMMENT ON COLUMN public.msg_target_category.name IS '名称
@len 1..=50';

COMMENT ON COLUMN public.msg_target_category.remark IS '备注
@len 0..=50';

COMMENT ON COLUMN public.msg_target_category.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_target_category.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_target_category.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_target_category.update_ms IS '更新时间戳';

CREATE TABLE public.msg_target_category_channel (
    id bigint NOT NULL,
    target_category_id bigint NOT NULL,
    channel_id bigint NOT NULL,
    creater_id bigint NOT NULL,
    create_ms bigint NOT NULL,
    updater_id bigint NOT NULL,
    update_ms bigint NOT NULL
);

COMMENT ON TABLE public.msg_target_category_channel IS '目标类别渠道';

COMMENT ON COLUMN public.msg_target_category_channel.id IS 'ID';

COMMENT ON COLUMN public.msg_target_category_channel.target_category_id IS '目标类别ID';

COMMENT ON COLUMN public.msg_target_category_channel.channel_id IS '渠道ID';

COMMENT ON COLUMN public.msg_target_category_channel.creater_id IS '创建者ID';

COMMENT ON COLUMN public.msg_target_category_channel.create_ms IS '创建时间戳';

COMMENT ON COLUMN public.msg_target_category_channel.updater_id IS '更新者ID';

COMMENT ON COLUMN public.msg_target_category_channel.update_ms IS '更新时间戳';

ALTER TABLE ONLY public.msg_target_category_channel
    ADD CONSTRAINT ak_channel_id_and_target_category_id_msg_target_category_channe UNIQUE (channel_id, target_category_id);

ALTER TABLE ONLY public.msg_channel
    ADD CONSTRAINT ak_code_msg_channel UNIQUE (code);

ALTER TABLE ONLY public.msg_delivery_queue
    ADD CONSTRAINT ak_code_msg_delivery_queue UNIQUE (code);

ALTER TABLE ONLY public.msg_event_source
    ADD CONSTRAINT ak_code_msg_event_source UNIQUE (code);

ALTER TABLE ONLY public.msg_message_category
    ADD CONSTRAINT ak_code_msg_message_category UNIQUE (code);

ALTER TABLE ONLY public.msg_message_queue
    ADD CONSTRAINT ak_code_msg_message_queue UNIQUE (code);

ALTER TABLE ONLY public.msg_target_category
    ADD CONSTRAINT ak_code_msg_target_category UNIQUE (code);

ALTER TABLE ONLY public.msg_delivery_channel
    ADD CONSTRAINT ak_deliver_target_id_and_channel_id_msg_delivery_channel UNIQUE (deliver_target_id, channel_id);

ALTER TABLE ONLY public.msg_delivery_target
    ADD CONSTRAINT ak_delivery_id_and_target_category_id_and_target_id_msg_deliver UNIQUE (delivery_id, target_category_id, target_id);

ALTER TABLE ONLY public.msg_delivery
    ADD CONSTRAINT ak_event_code_and_business_id_and_business_trigger_ms_msg_deliv UNIQUE (event_code, business_id, business_trigger_ms);

ALTER TABLE ONLY public.msg_message
    ADD CONSTRAINT ak_event_code_msg_message UNIQUE (event_code);

ALTER TABLE ONLY public.msg_message_target
    ADD CONSTRAINT ak_message_id_and_target_category_id_and_target_id_msg_message_ UNIQUE (message_id, target_category_id, target_id);

ALTER TABLE ONLY public.msg_delivery_queue
    ADD CONSTRAINT ak_name_msg_delivery_queue UNIQUE (name);

ALTER TABLE ONLY public.msg_event_source
    ADD CONSTRAINT ak_name_msg_event_source UNIQUE (name);

ALTER TABLE ONLY public.msg_message
    ADD CONSTRAINT ak_name_msg_message UNIQUE (name);

ALTER TABLE ONLY public.msg_message_category
    ADD CONSTRAINT ak_name_msg_message_category UNIQUE (name);

ALTER TABLE ONLY public.msg_message_queue
    ADD CONSTRAINT ak_name_msg_message_queue UNIQUE (name);

ALTER TABLE ONLY public.msg_target_category
    ADD CONSTRAINT ak_name_msg_target_category UNIQUE (name);

ALTER TABLE ONLY public.msg_channel
    ADD CONSTRAINT pk_msg_channel PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_dead_letter
    ADD CONSTRAINT pk_msg_dead_letter PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_delivery
    ADD CONSTRAINT pk_msg_delivery PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_delivery_channel
    ADD CONSTRAINT pk_msg_delivery_channel PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_delivery_channel_log
    ADD CONSTRAINT pk_msg_delivery_channel_log PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_delivery_queue
    ADD CONSTRAINT pk_msg_delivery_queue PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_delivery_target
    ADD CONSTRAINT pk_msg_delivery_target PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_event_source
    ADD CONSTRAINT pk_msg_event_source PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_message
    ADD CONSTRAINT pk_msg_message PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_message_category
    ADD CONSTRAINT pk_msg_message_category PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_message_queue
    ADD CONSTRAINT pk_msg_message_queue PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_message_target
    ADD CONSTRAINT pk_msg_message_target PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_target_category
    ADD CONSTRAINT pk_msg_target_category PRIMARY KEY (id);

ALTER TABLE ONLY public.msg_target_category_channel
    ADD CONSTRAINT pk_msg_target_category_channel PRIMARY KEY (id);

CREATE UNIQUE INDEX msg_channel_pk ON public.msg_channel USING btree (id);

CREATE UNIQUE INDEX msg_dead_letter_pk ON public.msg_dead_letter USING btree (id);

CREATE UNIQUE INDEX msg_delivery_channel_log_pk ON public.msg_delivery_channel_log USING btree (id);

CREATE UNIQUE INDEX msg_delivery_channel_pk ON public.msg_delivery_channel USING btree (id);

CREATE UNIQUE INDEX msg_delivery_pk ON public.msg_delivery USING btree (id);

CREATE UNIQUE INDEX msg_delivery_queue_pk ON public.msg_delivery_queue USING btree (id);

CREATE UNIQUE INDEX msg_delivery_target_pk ON public.msg_delivery_target USING btree (id);

CREATE UNIQUE INDEX msg_event_source_pk ON public.msg_event_source USING btree (id);

CREATE UNIQUE INDEX msg_message_category_pk ON public.msg_message_category USING btree (id);

CREATE UNIQUE INDEX msg_message_pk ON public.msg_message USING btree (id);

CREATE UNIQUE INDEX msg_message_queue_pk ON public.msg_message_queue USING btree (id);

CREATE UNIQUE INDEX msg_message_target_pk ON public.msg_message_target USING btree (id);

CREATE UNIQUE INDEX msg_target_category_channel_pk ON public.msg_target_category_channel USING btree (id);

CREATE UNIQUE INDEX msg_target_category_pk ON public.msg_target_category USING btree (id);

CREATE INDEX relationship_10_fk ON public.msg_delivery_channel_log USING btree (delivery_channel_id);

CREATE INDEX relationship_11_fk ON public.msg_message_target USING btree (target_category_id);

CREATE INDEX relationship_12_fk ON public.msg_delivery_target USING btree (target_category_id);

CREATE INDEX relationship_13_fk ON public.msg_message USING btree (event_source_id);

CREATE INDEX relationship_14_fk ON public.msg_target_category_channel USING btree (target_category_id);

CREATE INDEX relationship_15_fk ON public.msg_target_category_channel USING btree (channel_id);

CREATE INDEX relationship_16_fk ON public.msg_delivery USING btree (event_source_id);

CREATE INDEX relationship_17_fk ON public.msg_delivery USING btree (message_category_id);

CREATE INDEX relationship_1_fk ON public.msg_message USING btree (category_id);

CREATE INDEX relationship_3_fk ON public.msg_delivery USING btree (message_id);

CREATE INDEX relationship_4_fk ON public.msg_message_target USING btree (message_id);

CREATE INDEX relationship_7_fk ON public.msg_delivery_target USING btree (delivery_id);

CREATE INDEX relationship_8_fk ON public.msg_delivery_channel USING btree (deliver_target_id);

CREATE INDEX relationship_9_fk ON public.msg_delivery_channel USING btree (channel_id);

ALTER TABLE ONLY public.msg_message
    ADD CONSTRAINT fk_category_id__from__msg_message_category FOREIGN KEY (category_id) REFERENCES public.msg_message_category(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery_channel
    ADD CONSTRAINT fk_channel_id__from__msg_channel FOREIGN KEY (channel_id) REFERENCES public.msg_channel(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_target_category_channel
    ADD CONSTRAINT fk_channel_id__from__msg_channel FOREIGN KEY (channel_id) REFERENCES public.msg_channel(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery_channel
    ADD CONSTRAINT fk_deliver_target_id__from__msg_delivery_target FOREIGN KEY (deliver_target_id) REFERENCES public.msg_delivery_target(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery_channel_log
    ADD CONSTRAINT fk_delivery_channel_id__from__msg_delivery_channel FOREIGN KEY (delivery_channel_id) REFERENCES public.msg_delivery_channel(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery_target
    ADD CONSTRAINT fk_delivery_id__from__msg_delivery FOREIGN KEY (delivery_id) REFERENCES public.msg_delivery(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery
    ADD CONSTRAINT fk_event_source_id__from__msg_event_source FOREIGN KEY (event_source_id) REFERENCES public.msg_event_source(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_message
    ADD CONSTRAINT fk_event_source_id__from__msg_event_source FOREIGN KEY (event_source_id) REFERENCES public.msg_event_source(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery
    ADD CONSTRAINT fk_message_category_id__from__msg_message_category FOREIGN KEY (message_category_id) REFERENCES public.msg_message_category(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery
    ADD CONSTRAINT fk_message_id__from__msg_message FOREIGN KEY (message_id) REFERENCES public.msg_message(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_message_target
    ADD CONSTRAINT fk_message_id__from__msg_message FOREIGN KEY (message_id) REFERENCES public.msg_message(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_delivery_target
    ADD CONSTRAINT fk_target_category_id__from__msg_target_category FOREIGN KEY (target_category_id) REFERENCES public.msg_target_category(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_message_target
    ADD CONSTRAINT fk_target_category_id__from__msg_target_category FOREIGN KEY (target_category_id) REFERENCES public.msg_target_category(id) ON UPDATE RESTRICT ON DELETE RESTRICT;

ALTER TABLE ONLY public.msg_target_category_channel
    ADD CONSTRAINT fk_target_category_id__from__msg_target_category FOREIGN KEY (target_category_id) REFERENCES public.msg_target_category(id) ON UPDATE RESTRICT ON DELETE RESTRICT;
