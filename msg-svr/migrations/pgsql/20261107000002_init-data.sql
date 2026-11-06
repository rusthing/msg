-- msg 数据库迁移基线：init-data（字典与入口种子）
--
-- 本文件由 A 库（重放历史全部迁移 + 生产补种）导出，用于替代历史迁移。
-- 一旦发布并写入 _sqlx_migrations，禁止再修改本文件；后续变更必须新增迁移。

INSERT INTO public.msg_channel VALUES (2, 'websocket', 'websocket', '{"token":""}', '@engine: robotech
{"eventCode":{{eventCode:json}},"data":{{ext:json}}}', 'WebSocket长连接', true, 0, 1793976041647, 0, 1793976041647) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_channel VALUES (3, 'locator_card', '定位卡渠道', NULL, NULL, '定位卡下行：Core NATS 广播到 dtw.card.callback', true, 0, 1793976042413, 0, 1793976042413) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_delivery_queue VALUES (1, 'msg.delivery.websocket', 'websocket 投递队列', false, 'WebSocket长连接广播，所有实例都收', 0, 1793976041685, 0, 1793976041685) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_delivery_queue VALUES (2, 'msg.delivery.locator_card', '定位卡投递队列', false, '定位卡下行 Core NATS 广播，所有实例都收', 0, 1793976042442, 0, 1793976042442) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_event_source VALUES (2, 'alarm', '报警中心', '报警事件来源：事件策略与存续期状态', 0, 1793976041825, 0, 1793976041825) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_event_source VALUES (1060348598531227660, 'dtw', '人员定位', 'dtw 定位与定位卡数据', 0, 1793976042901, 0, 1793976042901) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_message_category VALUES (1, 'normal', '普通消息', '默认类别：未特别区分类别的业务消息', 0, 1793976041632, 0, 1793976041632) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_category VALUES (2, 'alarm', '报警', '报警中心发出的报警通知', 0, 1793976042007, 0, 1793976042007) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_category VALUES (3, 'locator_card', '定位卡', '定位卡下行通知的消息类别', 0, 1793976042391, 0, 1793976042391) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_category VALUES (1060348598584311827, 'telemetry', '遥测', '设备上报的遥测消息', 0, 1793976042912, 0, 1793976042912) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_message VALUES (2, 2, 1, '报警状态', 'alarm.alert.status', false, '报警状态：{{eventCode:default(报警)}}', '区域{{regionName:default(未知区域)}} 事件{{eventCode}} 级别{{alertLevelId}} 人数{{regionCount:default(0)}} 持续{{durationMs:default(0)}}ms 状态{{alertStatus}}', '{"alertId":{{alertId}},"eventCode":{{eventCode:json}},"alertStatus":{{alertStatus:json}},"alertLevelId":{{alertLevelId}},"regionId":{{regionId:default(0)}},"regionCount":{{regionCount:default(0)}},"durationMs":{{durationMs:default(0)}},"reportMs":{{reportMs}}}', '报警存续期状态推大屏：区域名、人数、事件、级别与持续时间', true, 0, 1793976041847, 0, 1793976041847, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (3, 2, 2, '区域禁入报警', 'alarm.dtw.region.enter.forbidden', true, NULL, NULL, NULL, '人员进入禁止区域', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (4, 2, 2, '区域禁出报警', 'alarm.dtw.region.exit.forbidden', true, NULL, NULL, NULL, '人员离开禁止区域', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (5, 2, 2, '区域静止报警', 'alarm.dtw.region.static', true, NULL, NULL, NULL, '人员在区域内静止超时', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (6, 2, 2, '区域滞留报警', 'alarm.dtw.region.dwell', true, NULL, NULL, NULL, '人员在区域内滞留超时', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (7, 2, 2, '区域人员聚集报警', 'alarm.dtw.region.gather', true, NULL, NULL, NULL, '区域内人数超过聚集阈值', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (8, 2, 2, '区域超员报警', 'alarm.dtw.region.overcrowd', true, NULL, NULL, NULL, '区域内人数超过容量上限', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (9, 2, 2, '区域缺员报警', 'alarm.dtw.region.undercrowd', true, NULL, NULL, NULL, '区域内人数低于容量下限', true, 0, 1793976042024, 0, 1793976042024, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (10, 2, 2, '区域禁入超时升级告警', 'alarm.dtw.region.enter.forbidden.escalated', true, NULL, NULL, NULL, '禁入首报 30 秒未消警升级：严重级，走大屏', true, 0, 1793976042700, 0, 1793976042700, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (11, 2, 2, '区域禁出超时升级告警', 'alarm.dtw.region.exit.forbidden.escalated', true, NULL, NULL, NULL, '禁出首报 30 秒未消警升级：严重级，走大屏', true, 0, 1793976042700, 0, 1793976042700, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (12, 2, 2, '定位卡 SOS 报警', 'alarm.dtw.locator.sos', true, '定位卡 {{locatorCode}} 触发 SOS 报警', '定位卡 {{locatorCode}} SOS 一键报警；所在区域 {{regionName:default(未知)}}；上报时间 {{triggerTimestamp:default(未知)}}', '{"alertId":{{alertId:json}},"eventCode":{{eventCode:json}},"alertStatus":{{alertStatus:json}},"locatorCode":{{locatorCode:json}},"ALARM_END_TIME":{{ALARM_END_TIME:json}},"DESCRIBE":{{DESCRIBE:json}}}', 'SOS 一键报警（附录 B 表 B.3）：补报警结束时间与描述，走大屏', true, 0, 1793976042728, 0, 1793976042728, false) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message VALUES (1060348598592163850, 1060348598531227660, 1060348598584311827, '位置更新', 'dtw.position.updated', false, '@engine: robotech
定位卡 {{locatorCode}} 位置已更新', '@engine: robotech
定位卡 {{locatorCode}} 位于 {{floor:default(未知)}} 层', '@engine: robotech
{"mode":{{mode}},"locatorCode":{{locatorCode:json}},"lon":{{lon}},"lat":{{lat}},"floor":{{floor:json}},"triggerTimestamp":{{triggerTimestamp}},"unityX":{{unityX}},"unityY":{{unityY}},"unityZ":{{unityZ}},"bindTargetType":{{bindTargetType}},"bindTargetId":{{bindTargetId}},"bindTargetName":{{bindTargetName:json}},"bindTargetSubType":{{bindTargetSubType:json}},"regionId":{{regionId:json}},"regionName":{{regionName:json}},"regionEvent":{{regionEvent:json}},"previousRegionId":{{previousRegionId:json}}}', NULL, true, 0, 1793976042929, 0, 1793976042929, false) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_message_queue VALUES (2, 'msg.publish.screen', '大屏状态消息队列', false, '报警存续期状态直推大屏', 0, 1793976041836, 0, 1793976041836) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348560335536129, 'msg.publish.alarm.1', '1级报警消息队列', true, '承载 1 级报警（最严重），持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348560340111361, 'msg.publish.alarm.2', '2级报警消息队列', true, '承载 2 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348560344080385, 'msg.publish.alarm.3', '3级报警消息队列', true, '承载 3 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348560346677250, 'msg.publish.alarm.4', '4级报警消息队列', true, '承载 4 级报警，持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348560349941762, 'msg.publish.alarm.5', '5级报警消息队列', true, '承载 5 级报警（最轻），持久化以免报警丢失', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060348598579068945, 'msg.publish.telemetry', '遥测消息队列', false, '设备上报的遥测普通消息', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_queue VALUES (1060456910744801280, 'msg.message.normal', '普通消息队列', true, 'msg-pub 默认发布入口', 0, 1793976042919, 0, 1793976042919) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_target_category VALUES (1, 'user', '用户', '按用户投递：目标ID是用户ID', 0, 1793976041657, 0, 1793976041657) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_target_category VALUES (2, 'role', '角色', '按角色投递：目标ID是用户ID，角色下的用户都全部投递', 0, 1793976041657, 0, 1793976041657) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_target_category VALUES (3, 'group', '分组', '按分组投递：目标ID是用户ID，分组内的用户都收', 0, 1793976041657, 0, 1793976041657) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_target_category VALUES (4, 'screen', '大屏', '自定义目标：大屏长连接', 0, 1793976041665, 0, 1793976041665) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_target_category VALUES (5, 'locator_card', '定位卡', '定位卡目标：target_id 为卡号，空串广播全部卡片', 0, 1793976042405, 0, 1793976042405) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_message_target VALUES (2, 2, 4, '', 0, 1793976041860, 0, 1793976041860) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (5, 5, 4, '', 0, 1793976042037, 0, 1793976042037) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (6, 6, 4, '', 0, 1793976042037, 0, 1793976042037) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (7, 7, 4, '', 0, 1793976042037, 0, 1793976042037) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (8, 8, 4, '', 0, 1793976042037, 0, 1793976042037) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (9, 9, 4, '', 0, 1793976042037, 0, 1793976042037) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (10, 3, 5, '', 0, 1793976042686, 0, 1793976042686) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (11, 4, 5, '', 0, 1793976042686, 0, 1793976042686) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (12, 7, 5, '', 0, 1793976042686, 0, 1793976042686) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (13, 10, 4, '', 0, 1793976042737, 0, 1793976042737) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (14, 11, 4, '', 0, 1793976042737, 0, 1793976042737) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (15, 12, 4, '', 0, 1793976042737, 0, 1793976042737) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_message_target VALUES (1060348598600695829, 1060348598592163850, 4, '', 0, 1793976042945, 0, 1793976042945) ON CONFLICT DO NOTHING;

INSERT INTO public.msg_target_category_channel VALUES (1, 4, 2, 0, 1793976041672, 0, 1793976041672) ON CONFLICT DO NOTHING;
INSERT INTO public.msg_target_category_channel VALUES (2, 5, 3, 0, 1793976042425, 0, 1793976042425) ON CONFLICT DO NOTHING;

