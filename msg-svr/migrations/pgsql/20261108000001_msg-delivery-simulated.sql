-- 投递记录增加「是否模拟」字段
--
-- 口径：发布方的消息 labels/annotations 带 simulated=true 或 source=sim 时为 true，
-- 与 msg-svr 现有模拟标记识别逻辑一致。历史数据无法回推，统一按默认值 false 处理。

ALTER TABLE public.msg_delivery
    ADD COLUMN IF NOT EXISTS simulated boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN public.msg_delivery.simulated IS '是否模拟：发布方带 simulated=true 或 source=sim 标记时为是';
