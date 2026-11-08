-- msg 数据库迁移基线：init-index（普通索引）
--
-- 约束型索引随建表语句定义；此处只放独立 CREATE INDEX（含部分/表达式索引）。

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
