-- 修正 dtw / msg 种子数据
--
-- 1. 位置更新事件编码统一为 telemetry.dtw.position.updated；
-- 2. 消息类别「定位卡」名称改为「遥控」；
-- 3. 目标类别、投递渠道编码统一为 locator，投递队列同步改主题末段；
-- 4. 禁入、禁出消息目标补「大屏」（聚集此前已配置，无需重复插入）；
-- 5. 定位卡投递队列由 Core NATS 广播改为 JetStream + 队列组。
--
-- 全部语句可重复执行：UPDATE 带旧值条件，INSERT 带 NOT EXISTS + ON CONFLICT。

-- 1. 位置更新事件编码（若新编码已被自动注册则跳过，避免唯一键冲突）
UPDATE public.msg_message
SET event_code = 'telemetry.dtw.position.updated',
    update_ms = 1794136800000
WHERE event_code = 'dtw.position.updated'
  AND NOT EXISTS (
      SELECT 1 FROM public.msg_message WHERE event_code = 'telemetry.dtw.position.updated'
  );

-- 2. 消息类别显示名：定位卡 → 遥控
UPDATE public.msg_message_category
SET name = '遥控',
    remark = '遥控下行通知的消息类别',
    update_ms = 1794136800000
WHERE id = 3
  AND code = 'locator_card';

-- 3.1 目标类别编码：locator_card → locator
UPDATE public.msg_target_category
SET code = 'locator',
    update_ms = 1794136800000
WHERE id = 5
  AND code = 'locator_card'
  AND NOT EXISTS (
      SELECT 1 FROM public.msg_target_category WHERE code = 'locator' AND id <> 5
  );

-- 3.2 投递渠道编码：locator_card → locator
UPDATE public.msg_channel
SET code = 'locator',
    remark = '定位卡下行：NATS 转 dtw.card.callback',
    update_ms = 1794136800000
WHERE id = 3
  AND code = 'locator_card'
  AND NOT EXISTS (
      SELECT 1 FROM public.msg_channel WHERE code = 'locator' AND id <> 3
  );

-- 3.3 投递队列主题末段跟随渠道编码，并改为 JetStream + 队列组（persisted=true）
UPDATE public.msg_delivery_queue
SET code = 'msg.delivery.locator',
    persisted = true,
    remark = '定位卡下行：JetStream 队列组负载均衡，单实例转发到 dtw.card.callback',
    update_ms = 1794136800000
WHERE id = 2
  AND code = 'msg.delivery.locator_card';

UPDATE public.msg_delivery_queue
SET persisted = true,
    remark = '定位卡下行：JetStream 队列组负载均衡，单实例转发到 dtw.card.callback',
    update_ms = 1794136800000
WHERE code = 'msg.delivery.locator';

-- 4. 禁入、禁出消息目标补大屏（target_category_id=4）
INSERT INTO public.msg_message_target
    (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms)
SELECT 1060348598600695830, 3, 4, '', 0, 1794136800000, 0, 1794136800000
WHERE NOT EXISTS (
    SELECT 1 FROM public.msg_message_target
    WHERE message_id = 3 AND target_category_id = 4 AND target_id = ''
)
ON CONFLICT DO NOTHING;

INSERT INTO public.msg_message_target
    (id, message_id, target_category_id, target_id, creater_id, create_ms, updater_id, update_ms)
SELECT 1060348598600695831, 4, 4, '', 0, 1794136800000, 0, 1794136800000
WHERE NOT EXISTS (
    SELECT 1 FROM public.msg_message_target
    WHERE message_id = 4 AND target_category_id = 4 AND target_id = ''
)
ON CONFLICT DO NOTHING;

-- 聚集（message_id=7）的大屏目标已存在于 20261107000002_init-data.sql 第 55 行，此处不重复插入。
