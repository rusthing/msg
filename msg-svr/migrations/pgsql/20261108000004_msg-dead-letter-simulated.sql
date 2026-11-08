-- 死信记录增加「是否模拟」字段
--
-- 口径：死信落库时从原始负载（能解析为 MessageMsg 时）按 labels/annotations 的
-- simulated=true 或 source=sim 标记判定，与 msg_delivery.simulated 一致。
-- 死信没有外键指向 msg_delivery/msg_message，必须由写入方携带，不做事后关联推断。
-- 历史数据 payload 结构异构（dlq.> 覆盖任意主题，且 DeliveryMsg 不含 labels），
-- 无法可靠回推，统一按默认值 false 处理。

ALTER TABLE public.msg_dead_letter
    ADD COLUMN IF NOT EXISTS simulated boolean NOT NULL DEFAULT false;

COMMENT ON COLUMN public.msg_dead_letter.simulated IS '是否模拟：死信落库时按原始负载的 simulated=true 或 source=sim 标记写入';
