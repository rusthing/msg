-- msg_channel
INSERT INTO public.msg_channel (id, code, name, options, payload_template, remark, enabled, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'websocket', 'websocket', '{"token":""}', '@engine: robotech
{"eventCode":{{eventCode:json}},"data":{{ext:json}},"firstTriggerMs":{{firstTriggerMs:default(null)}},"firstReceiveMs":{{firstReceiveMs:default(null)}},"receiveMs":{{receiveMs:default(null)}},"businessTriggerMs":{{businessTriggerMs:default(null)}},"sendMs":{{sendMs:default(null)}}}', 'WebSocket长连接', true, 0, 1793976041647, 0, 1793976041647);
INSERT INTO public.msg_channel (id, code, name, options, payload_template, remark, enabled, creater_id, create_ms, updater_id, update_ms) VALUES (3, 'locator', '定位卡渠道', NULL, NULL, '定位卡下行：NATS 转 dtw.card.callback', true, 0, 1793976042413, 0, 1794136800000);

-- 短信渠道：网关参数存 options（运营改渠道表即可换网关，不用重启）
--   gatewayUrl  网关下发地址（POST JSON）
--   appId       网关分配的账号
--   appKey      网关分配的密钥
--   sign        短信签名，拼在正文前
--   templateId  模板短信的模板编号，空串表示按正文直发
-- payload_template 留空：短信正文直接用消息 content（content 为空的消息在入队前就被拒收）
INSERT INTO public.msg_channel (id, code, name, options, payload_template, remark, enabled, creater_id, create_ms, updater_id, update_ms) VALUES (4, 'sms', '短信渠道', '{"gatewayUrl":"","appId":"","appKey":"","sign":"","templateId":""}', NULL, '短信网关逐条下发，按手机号发送', true, 0, 1794136800000, 0, 1794136800000);

-- 邮件渠道：SMTP 参数同样存 options
--   smtpHost/smtpPort  服务器与端口（465 走隐式 TLS，587 走 STARTTLS）
--   username/password  认证账号
--   fromAddress        发信人，留空时取 username
--   ssl                true 走 TLS
-- 主题取消息 title，正文取 content
INSERT INTO public.msg_channel (id, code, name, options, payload_template, remark, enabled, creater_id, create_ms, updater_id, update_ms) VALUES (5, 'email', '邮件渠道', '{"smtpHost":"","smtpPort":465,"username":"","password":"","fromAddress":"","ssl":true}', NULL, 'SMTP 直发，按邮箱地址发送', true, 0, 1794136800000, 0, 1794136800000);

-- msg_event_source
INSERT INTO public.msg_event_source (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'alarm', '报警中心', '报警事件来源：事件策略与存续期状态', 0, 1793976041825, 0, 1793976041825);
INSERT INTO public.msg_event_source (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598531227660, 'dtw', '人员定位', 'dtw 定位与定位卡数据', 0, 1793976042901, 0, 1793976042901);

-- msg_message_category
INSERT INTO public.msg_message_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1, 'normal', '普通消息', '默认类别：未特别区分类别的业务消息', 0, 1793976041632, 0, 1793976041632);
INSERT INTO public.msg_message_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'alarm', '报警', '报警中心发出的报警通知', 0, 1793976042007, 0, 1793976042007);
INSERT INTO public.msg_message_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598584311827, 'telemetry', '遥测', '设备上报的遥测消息', 0, 1793976042912, 0, 1793976042912);
INSERT INTO public.msg_message_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (3, 'locator_card', '遥控', '遥控下行通知的消息类别', 0, 1793976042391, 0, 1794136800000);

-- msg_message
--
-- 报警中心（event_source=alarm）发出的消息，标题/正文/ext 在报警中心那一侧
-- 已经按事件策略渲染完（见 alarm-svr 的 `msg::alarm_alert_msg`），登记到消息
-- 中心时三份模板必须留空：`message_sub::pick` 是「模板优先」，一旦非空就会用
-- 消息中心手里（基本为空）的变量重渲染，把 alertId / 坐标 / alertStatus /
-- alertEffects 全部冲成 null——大屏收到的帧能到，但内容不可用，而且不报错。
--
-- 例外是 `alarm.alert.status` 与 `telemetry.dtw.position.updated`：这两条发布方
-- 只给结构化字段（labels/annotations），文案本来就该由消息中心渲染。
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (2, 2, 1, '报警状态', 'alarm.alert.status', false, '报警状态：{{eventCode:default(报警)}}', '区域{{regionName:default(未知区域)}} 事件{{eventCode}} 级别{{alertLevelId}} 人数{{regionCount:default(0)}} 持续{{durationMs:default(0)}}ms 状态{{alertStatus}}', '{"alertId":{{alertId}},"eventCode":{{eventCode:json}},"alertStatus":{{alertStatus:json}},"alertLevelId":{{alertLevelId}},"regionId":{{regionId:default(0)}},"regionCount":{{regionCount:default(0)}},"durationMs":{{durationMs:default(0)}},"reportMs":{{reportMs}}}', '报警存续期状态推大屏：区域名、人数、事件、级别与持续时间', true, 0, 1793976041847, 0, 1793976041847, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (7, 2, 2, '区域人员聚集报警', 'alarm.dtw.gather', true, NULL, NULL, NULL, '区域内人数超过聚集阈值', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (1060348598592163850, 1060348598531227660, 1060348598584311827, '位置更新', 'telemetry.dtw.position.updated', false, '@engine: robotech
定位卡 {{locatorCode}} 位置已更新', '@engine: robotech
定位卡 {{locatorCode}} 位于 {{floor:default(未知)}} 层', '@engine: robotech
{"mode":{{mode}},"locatorCode":{{locatorCode:json}},"lon":{{lon}},"lat":{{lat}},"floor":{{floor:json}},"triggerMs":{{triggerMs}},"unityX":{{unityX}},"unityY":{{unityY}},"unityZ":{{unityZ}},"bindTargetType":{{bindTargetType}},"bindTargetId":{{bindTargetId}},"bindTargetName":{{bindTargetName:json}},"bindTargetSubType":{{bindTargetSubType:json}},"regionId":{{regionId:json}},"regionName":{{regionName:json}},"regionEvent":{{regionEvent:json}},"previousRegionId":{{previousRegionId:json}}}', NULL, true, 0, 1793976042929, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (6, 2, 2, '区域滞留报警', 'alarm.dtw.dwell', true, NULL, NULL, NULL, '人员在区域内滞留超时', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (3, 2, 2, '区域禁入报警', 'alarm.dtw.enter', true, NULL, NULL, NULL, '人员进入禁止区域', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (12, 2, 2, '定位卡 SOS 报警', 'alarm.dtw.sos', true, NULL, NULL, NULL, 'SOS 一键报警：模板由报警策略渲染，msg 侧留空', true, 0, 1793976042728, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (4, 2, 2, '区域禁出报警', 'alarm.dtw.exit', true, NULL, NULL, NULL, '人员离开禁止区域', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (10, 2, 2, '区域禁入超时升级告警', 'alarm.dtw.enter.escalated', true, NULL, NULL, NULL, '禁入首报 30 秒未消警升级：严重级，走大屏', true, 0, 1793976042700, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (11, 2, 2, '区域禁出超时升级告警', 'alarm.dtw.exit.escalated', true, NULL, NULL, NULL, '禁出首报 30 秒未消警升级：严重级，走大屏', true, 0, 1793976042700, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (8, 2, 2, '区域超员报警', 'alarm.dtw.overcrowd', true, NULL, NULL, NULL, '区域内人数超过容量上限', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (5, 2, 2, '区域静止报警', 'alarm.dtw.static', true, NULL, NULL, NULL, '人员在区域内静止超时', true, 0, 1793976042024, 0, 1794136800000, true);
INSERT INTO public.msg_message (id, event_source_id, category_id, name, event_code, persisted, title_template, content_template, ext_template, remark, enabled, creater_id, create_ms, updater_id, update_ms, sim_enabled) VALUES (9, 2, 2, '区域缺员报警', 'alarm.dtw.undercrowd', true, NULL, NULL, NULL, '区域内人数低于容量下限', true, 0, 1793976042024, 0, 1794136800000, true);

-- msg_target_category
INSERT INTO public.msg_target_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1, 'user', '用户', '按用户投递：目标ID是用户ID', 0, 1793976041657, 0, 1793976041657);
INSERT INTO public.msg_target_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'role', '角色', '按角色投递：目标ID是用户ID，角色下的用户都全部投递', 0, 1793976041657, 0, 1793976041657);
INSERT INTO public.msg_target_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (3, 'org', '组织', '按组织投递：目标ID是组织ID，含下级（tree_code 前缀匹配）', 0, 1793976041657, 0, 1793976041657);
INSERT INTO public.msg_target_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (4, 'screen', '大屏', '自定义目标：大屏长连接', 0, 1793976041665, 0, 1793976041665);
INSERT INTO public.msg_target_category (id, code, name, remark, creater_id, create_ms, updater_id, update_ms) VALUES (5, 'locator', '定位卡', '定位卡目标：target_id 为卡号，空串广播全部卡片', 0, 1793976042405, 0, 1794136800000);

-- msg_delivery_queue
INSERT INTO public.msg_delivery_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1, 'msg.delivery.websocket', 'websocket 投递队列', false, 'WebSocket长连接广播，所有实例都收', 0, 1793976041685, 0, 1793976041685);
INSERT INTO public.msg_delivery_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'msg.delivery.locator', '定位卡投递队列', true, '定位卡下行：JetStream 队列组负载均衡，单实例转发到 dtw.card.callback', 0, 1793976042442, 0, 1794136800000);
INSERT INTO public.msg_delivery_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (3, 'msg.delivery.sms', '短信投递队列', true, '短信不能丢：JetStream 队列组负载均衡，单实例调网关，失败可重投', 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_delivery_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (4, 'msg.delivery.email', '邮件投递队列', true, '邮件不能丢：JetStream 队列组负载均衡，单实例走 SMTP，失败可重投', 0, 1794136800000, 0, 1794136800000);

-- msg_message_queue
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (2, 'msg.publish.screen', '大屏状态消息队列', false, '报警存续期状态直推大屏', 0, 1793976041836, 0, 1793976041836);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348560335536129, 'msg.publish.alarm.1', '1级报警消息队列', true, '承载 1 级报警（最严重），持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348560340111361, 'msg.publish.alarm.2', '2级报警消息队列', true, '承载 2 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348560344080385, 'msg.publish.alarm.3', '3级报警消息队列', true, '承载 3 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348560346677250, 'msg.publish.alarm.4', '4级报警消息队列', true, '承载 4 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348560349941762, 'msg.publish.alarm.5', '5级报警消息队列', true, '承载 5 级报警（最轻），持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598579068945, 'msg.publish.telemetry', '遥测消息队列', false, '设备上报的遥测普通消息', 0, 1793976042919, 0, 1793976042919);
INSERT INTO public.msg_message_queue (id, code, name, persisted, remark, creater_id, create_ms, updater_id, update_ms) VALUES (1060456910744801280, 'msg.message.normal', '普通消息队列', true, 'msg-pub 默认发布入口', 0, 1793976042919, 0, 1793976042919);

-- msg_message_target
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (2, 2, 4, '', 0, 1793976041860, 0, 1793976041860);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (5, 5, 4, '', 0, 1793976042037, 0, 1793976042037);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (6, 6, 4, '', 0, 1793976042037, 0, 1793976042037);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (7, 7, 4, '', 0, 1793976042037, 0, 1793976042037);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (8, 8, 4, '', 0, 1793976042037, 0, 1793976042037);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (9, 9, 4, '', 0, 1793976042037, 0, 1793976042037);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (10, 3, 5, '', 0, 1793976042686, 0, 1793976042686);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (11, 4, 5, '', 0, 1793976042686, 0, 1793976042686);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (12, 7, 5, '', 0, 1793976042686, 0, 1793976042686);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (13, 10, 4, '', 0, 1793976042737, 0, 1793976042737);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (14, 11, 4, '', 0, 1793976042737, 0, 1793976042737);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (15, 12, 4, '', 0, 1793976042737, 0, 1793976042737);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598600695829, 1060348598592163850, 4, '', 0, 1793976042945, 0, 1793976042945);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598600695830, 3, 4, '', 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (1060348598600695831, 4, 4, '', 0, 1794136800000, 0, 1794136800000);

-- 内置系统管理员角色（role_id=2）默认接收报警状态 websocket 推送
INSERT INTO public.msg_message_target (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms) VALUES (16, 2, 2, '2', 0, 1793976041860, 0, 1793976041860);

-- msg_target_category_channel
-- 只登记「这个渠道对这个类别真的能投」的组合，UI 新建目标时按它预填：
--   类别 4（大屏）→ 渠道 2（websocket）：大屏长连接上报的类别编码就是 screen
--   类别 5（定位卡）→ 渠道 3（locator）：走卡片直连
-- user / role / org 暂时没有可用渠道：websocket 发送器按「目标类别编码」找本实例
-- 连接，用户、角色、组织这三类没有任何客户端会连上来，绑 websocket 只会让每个
-- 具体目标都落 Failed（2026-11-21 生产验证：org 目标展开 524 人全部 Failed）。
-- 等 S4 接入短信/邮件后，这三类再绑 sms / email。
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (1, 4, 2, 0, 1793976041672, 0, 1793976041672);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (2, 5, 3, 0, 1793976042425, 0, 1793976042425);
-- user / role / org 走短信 + 邮件：这三类都是有手机号/邮箱的具体人，websocket 投不到（见 S2 点火结论）
-- 同一类别绑多个渠道时，展开阶段按渠道各自取地址：取不到的那一路跳过，全都没有才落 Skipped
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (3, 1, 4, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (4, 2, 4, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (5, 3, 4, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (6, 1, 5, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (7, 2, 5, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_target_category_channel (id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (8, 3, 5, 0, 1794136800000, 0, 1794136800000);

-- msg_message_target_channel
-- 投递只认这张表：上面的 msg_target_category_channel 只是「默认渠道集」，
-- 新建消息目标时由 UI 预填，不参与投递。
-- 大屏目标（类别 4）→ websocket；定位卡目标（类别 5）→ locator。
-- 下面 id=16 / id=17 那两行是「报警状态」消息上的系统管理员角色目标（类别 2）：同一目标绑
-- 短信（4）+ 邮件（5）。角色里的人不会连大屏，历史上绑过 websocket 那一路永远投不到，
-- 换成这两个需要地址的渠道后，没手机号/没邮箱的成员各自那一路跳过，不拖累其他成员。
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (1, 2, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (2, 5, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (3, 6, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (4, 7, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (5, 8, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (6, 9, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (7, 10, 3, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (8, 11, 3, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (9, 12, 3, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (10, 13, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (11, 14, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (12, 15, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (13, 1060348598600695829, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (14, 1060348598600695830, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (15, 1060348598600695831, 2, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (16, 16, 4, 0, 1794136800000, 0, 1794136800000);
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (17, 16, 5, 0, 1794136800000, 0, 1794136800000);