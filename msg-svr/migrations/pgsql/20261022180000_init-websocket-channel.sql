/*==============================================================*/
/* 初始化 websocket 渠道                                          */
/*==============================================================*/

-- 大屏通过长连接收消息，渠道必须先存在：投递侧按 `msg_channel.code` 拼主题
-- （`msg.delivery.websocket`），执行器也按同一个 code 找发送器。
-- 渠道表没这条记录时，握手会直接返回 503（连上也收不到消息）。
--
-- `options` 是握手校验用的配置：
--   {"token": "xxxx"}  → 大屏连接时 ?token=xxxx 必须一致
--   {"token": ""}      → 不校验（联调用）
-- 换令牌只要更新这个字段，不用重启服务（每次握手现查）。
--
-- id 固定为 2：id-worker 生成的是雪花 ID（长整型大数），不会与固定小 ID 冲突；
-- `on conflict do nothing` 兼容已手工插过同名渠道的库，重复执行不报错。
insert into msg_channel (
   id, code, name, options, remark, enabled, creater_id, create_ms, updater_id, update_ms
) values (
   2,
   'websocket',
   'websocket',
   '{"token":""}',
   '大屏长连接',
   true,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;
