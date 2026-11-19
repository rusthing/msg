-- 新增迁移，不改历史文件
--
-- msg_message_target_channel 的初始绑定：按迁移前「目标类别 → 渠道」的映射
-- 逐条展开到 16 个目标，行为与迁移前保持一致。
-- 新库（生产）里目标已经有渠道了，不会出现「有目标零渠道」。

-- msg_message_target_channel
-- 投递只认这张表：上面的 msg_target_category_channel 只是「默认渠道集」，
-- 新建消息目标时由 UI 预填，不参与投递。
-- 下面按迁移前「目标类别 → 渠道」的映射逐条展开到每个目标，行为与迁移前一致：
--   类别 4（大屏）/ 类别 2（角色）→ 渠道 2（websocket）
--   类别 5（定位卡）→ 渠道 3（locator）
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
INSERT INTO public.msg_message_target_channel (id, message_target_id, channel_id, creater_id, create_ms, updater_id, update_ms) VALUES (16, 16, 2, 0, 1794136800000, 0, 1794136800000);
