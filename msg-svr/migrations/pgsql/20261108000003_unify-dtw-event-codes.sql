-- dtw 报警事件编码统一：去掉第 3 层（region / locator），禁入/禁出去掉 forbidden
--
-- 纯重命名，不改变事件语义。SOS 由 alarm.dtw.locator.sos 改为 alarm.dtw.sos，
-- 与其它事件同一条规则：第 3 层只承载「大类」，不再保留 locator。
--
-- 幂等：UPDATE 只命中旧编码，重复执行无副作用。

-- 1. 消息中心注册的消息编码
WITH mapping(old_code, new_code) AS (
    VALUES
        ('alarm.dtw.region.enter.forbidden', 'alarm.dtw.enter'),
        ('alarm.dtw.region.exit.forbidden', 'alarm.dtw.exit'),
        ('alarm.dtw.region.static', 'alarm.dtw.static'),
        ('alarm.dtw.region.dwell', 'alarm.dtw.dwell'),
        ('alarm.dtw.region.gather', 'alarm.dtw.gather'),
        ('alarm.dtw.region.overcrowd', 'alarm.dtw.overcrowd'),
        ('alarm.dtw.region.undercrowd', 'alarm.dtw.undercrowd'),
        ('alarm.dtw.region.enter.forbidden.escalated', 'alarm.dtw.enter.escalated'),
        ('alarm.dtw.region.exit.forbidden.escalated', 'alarm.dtw.exit.escalated'),
        ('alarm.dtw.locator.sos', 'alarm.dtw.sos')
)
UPDATE public.msg_message m
SET event_code = mapping.new_code,
    update_ms = 1794136800000
FROM mapping
WHERE m.event_code = mapping.old_code
  AND NOT EXISTS (
      SELECT 1 FROM public.msg_message x WHERE x.event_code = mapping.new_code
  );

-- 2. 投递记录历史编码做映射，保证按新编码筛选能查到历史投递
WITH mapping(old_code, new_code) AS (
    VALUES
        ('alarm.dtw.region.enter.forbidden', 'alarm.dtw.enter'),
        ('alarm.dtw.region.exit.forbidden', 'alarm.dtw.exit'),
        ('alarm.dtw.region.static', 'alarm.dtw.static'),
        ('alarm.dtw.region.dwell', 'alarm.dtw.dwell'),
        ('alarm.dtw.region.gather', 'alarm.dtw.gather'),
        ('alarm.dtw.region.overcrowd', 'alarm.dtw.overcrowd'),
        ('alarm.dtw.region.undercrowd', 'alarm.dtw.undercrowd'),
        ('alarm.dtw.region.enter.forbidden.escalated', 'alarm.dtw.enter.escalated'),
        ('alarm.dtw.region.exit.forbidden.escalated', 'alarm.dtw.exit.escalated'),
        ('alarm.dtw.locator.sos', 'alarm.dtw.sos')
)
UPDATE public.msg_delivery d
SET event_code = mapping.new_code,
    update_ms = 1794136800000
FROM mapping
WHERE d.event_code = mapping.old_code
  AND NOT EXISTS (
      SELECT 1 FROM public.msg_delivery x
      WHERE x.event_code = mapping.new_code
        AND x.business_id = d.business_id
        AND x.business_trigger_ms = d.business_trigger_ms
  );
