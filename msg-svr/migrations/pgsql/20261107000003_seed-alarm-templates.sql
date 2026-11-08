-- msg 数据库迁移：补齐 dtw 报警事件的消息模板
--
-- 背景：`20261107000002_init-data.sql` 已发布并写入 `_sqlx_migrations`，不再修改；
-- 这里用新的迁移把区域报警各事件的 title/content/ext 补全。
--
-- 变量来源：dtw 在报警事件的 `labels`（结构化事实）里注入，模板引擎对缺失/空白
-- 变量按 `default(...)` 兜底。各事件可用变量见 dtw-svr
-- `src/svc/dtw_region_alarm_svc.rs` 的 `forbidden_labels` / `card_labels` /
-- `region_labels` / `with_person_labels`：
--
--   禁入、禁出及其升级：locatorCode、cardCount、locatorCodes、regionId、
--     regionName、currentRegionId、triggerTimestamp（升级报警复用原报警的
--     labels/annotations，见 alarm-svr `build_escalate_trigger_dto`）
--   静止：locatorCode、regionId、regionName、triggerTimestamp、durationMs、radiusM
--   滞留：locatorCode、regionId、regionName、triggerTimestamp、durationMs
--   聚集：regionId、regionName、count、triggerTimestamp、radiusM、durationMs、
--     gatherLevel、clusterCenter、cardCount、cardCodes、personCount、personIds
--   超员：regionId、regionName、count、triggerTimestamp、limit、cardCount、
--     cardCodes、personCount、personIds
--   缺员：regionId、regionName、count、triggerTimestamp、limit（可能缺省）、
--     rosterAbsent、cardCount、cardCodes、personCount、personIds
--
-- ext 的用途：websocket 渠道把它直接拼进 `{"eventCode":...,"data":{{ext:json}}}`
-- （见 `20261107000002_init-data.sql` 的 msg_channel 定义），因此必须是合法 JSON；
-- 这里按报警载荷结构写，供大屏/前端直接消费。

UPDATE public.msg_message
SET title_template = '区域禁入报警：{{regionName:default(未知区域)}}',
    content_template = '人员 {{locatorCode}}（共 {{cardCount:default(1)}} 人）进入禁入区域 {{regionName:default(未知区域)}}；当前所在区域 {{currentRegionId:default(无)}}；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"locatorCodes":{{locatorCodes:json}},"cardCount":{{cardCount:default(1)}},"currentRegionId":{{currentRegionId:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.enter.forbidden';

UPDATE public.msg_message
SET title_template = '区域禁出报警：{{regionName:default(未知区域)}}',
    content_template = '人员 {{locatorCode}}（共 {{cardCount:default(1)}} 人）离开禁出区域 {{regionName:default(未知区域)}}；当前所在区域 {{currentRegionId:default(无)}}；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"locatorCodes":{{locatorCodes:json}},"cardCount":{{cardCount:default(1)}},"currentRegionId":{{currentRegionId:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.exit.forbidden';

UPDATE public.msg_message
SET title_template = '区域静止报警：{{regionName:default(未知区域)}}',
    content_template = '人员 {{locatorCode}} 在区域 {{regionName:default(未知区域)}} 静止超过 {{durationMs:default(0)}}ms（允许漂移 {{radiusM:default(0)}}m）；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"durationMs":{{durationMs:default(0)}},"radiusM":{{radiusM:default(0)}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.static';

UPDATE public.msg_message
SET title_template = '区域滞留报警：{{regionName:default(未知区域)}}',
    content_template = '人员 {{locatorCode}} 在区域 {{regionName:default(未知区域)}} 滞留超过 {{durationMs:default(0)}}ms；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"durationMs":{{durationMs:default(0)}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.dwell';

UPDATE public.msg_message
SET title_template = '区域人员聚集报警：{{regionName:default(未知区域)}}',
    content_template = '区域 {{regionName:default(未知区域)}} 聚集人数 {{count:default(0)}} 人（档位 {{gatherLevel:default(未知)}}，半径 {{radiusM:default(0)}}m，持续 {{durationMs:default(0)}}ms）；聚集中心 {{clusterCenter:default(未知)}}；涉及人员 {{personCount:default(0)}} 人（{{personIds:default(无)}}）；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"count":{{count:default(0)}},"gatherLevel":{{gatherLevel:json}},"radiusM":{{radiusM:default(0)}},"durationMs":{{durationMs:default(0)}},"clusterCenter":{{clusterCenter:json}},"personCount":{{personCount:default(0)}},"personIds":{{personIds:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.gather';

UPDATE public.msg_message
SET title_template = '区域超员报警：{{regionName:default(未知区域)}}',
    content_template = '区域 {{regionName:default(未知区域)}} 当前 {{count:default(0)}} 人，超过上限 {{limit:default(未配置)}} 人；涉及人员 {{personCount:default(0)}} 人（{{personIds:default(无)}}）；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"count":{{count:default(0)}},"limit":{{limit:default(0)}},"personCount":{{personCount:default(0)}},"personIds":{{personIds:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.overcrowd';

UPDATE public.msg_message
SET title_template = '区域缺员报警：{{regionName:default(未知区域)}}',
    content_template = '区域 {{regionName:default(未知区域)}} 当前 {{count:default(0)}} 人，低于下限 {{limit:default(未配置)}} 人；名单缺席 {{rosterAbsent:default(false)}}；涉及人员 {{personCount:default(0)}} 人（{{personIds:default(无)}}）；触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"count":{{count:default(0)}},"limit":{{limit:default(0)}},"rosterAbsent":{{rosterAbsent:default(false)}},"personCount":{{personCount:default(0)}},"personIds":{{personIds:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.undercrowd';

UPDATE public.msg_message
SET title_template = '区域禁入超时升级告警：{{regionName:default(未知区域)}}',
    content_template = '禁入报警超过 30 秒未消警，已升级为严重级；区域 {{regionName:default(未知区域)}}，人员 {{locatorCode}}（共 {{cardCount:default(1)}} 人），当前所在区域 {{currentRegionId:default(无)}}，首次触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"locatorCodes":{{locatorCodes:json}},"cardCount":{{cardCount:default(1)}},"currentRegionId":{{currentRegionId:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.enter.forbidden.escalated';

UPDATE public.msg_message
SET title_template = '区域禁出超时升级告警：{{regionName:default(未知区域)}}',
    content_template = '禁出报警超过 30 秒未消警，已升级为严重级；区域 {{regionName:default(未知区域)}}，人员 {{locatorCode}}（共 {{cardCount:default(1)}} 人），当前所在区域 {{currentRegionId:default(无)}}，首次触发时间 {{triggerTimestamp:default(未知)}}',
    ext_template = '{"eventCode":{{eventCode:json}},"regionId":{{regionId:default(0)}},"regionName":{{regionName:json}},"locatorCode":{{locatorCode:json}},"locatorCodes":{{locatorCodes:json}},"cardCount":{{cardCount:default(1)}},"currentRegionId":{{currentRegionId:json}},"triggerTimestamp":{{triggerTimestamp:default(0)}}}'
WHERE event_code = 'alarm.dtw.region.exit.forbidden.escalated';
