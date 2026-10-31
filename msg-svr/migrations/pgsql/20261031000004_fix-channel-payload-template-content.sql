-- 修正 websocket 渠道的载荷模板：把 title / content 补回去
--
-- 20261031000003 把默认帧落成了可读模板，但字段集少了两项，导致模板接管之后
-- 反而**比不配模板时少信息**——模板存在就不再走默认帧（见
-- msg-svr/src/wkr/channel/websocket.rs 的 `frame()`），于是：
--
--   默认帧：{eventCode, title, content, ext?, firstReceiveMs?, businessTriggerMs?}
--   旧模板：{eventCode, ext, firstReceiveMs, businessTriggerMs}
--
-- 丢掉的 `title` / `content` 是消息的可读部分：位置消息和 SOS 的标题都是一行
-- 文案，`content` 都是「定位卡 XXX 在哪 / 出了什么事」这句人话。模板一旦存在
-- 就不再走默认帧，缺字段就意味着这些消息在渠道上永久少东西。
--
-- （机器数据——坐标 / 楼层 / Unity 坐标 / 绑定目标——统一走 `ext`，位置消息见
-- dtw 的 `POSITION_EXT_TEMPLATE_DEFAULT`。`ext` 是「格式不约定」的载荷槽，
-- `content` 是给人看的正文，两者分工不再混用。）
--
-- 修正后的字段集与默认帧完全一致，避免再出现「配了模板比不配还差」：
--
--   title   一行人可读的文案（位置消息是「定位卡 XXX 位置已更新」，SOS 是
--           「定位卡 XXX 触发 SOS 报警」），大屏可以不上屏，但不该收不到；
--   content 消息内容：一句人话。**必须带 `:json`**——content 是文本，裸写
--           `{content}` 会把未转义的中文直接插进 JSON 字符串位置，整帧变成
--             {"eventCode":"dtw.locator.sos","content":定位卡 CARD-P-001 触发…}
--           解析失败后 `frame()` 只能回退默认帧（还每帧刷一条 WARN）。加了
--           `:json` 才是 `"content":"定位卡 CARD-P-001 触发…"`。
--
-- 只改仍然等于旧种子的那一版：运营按现场协议改过的模板不覆盖。判据是
-- 「字段集与旧种子一致」，用 md5 比对会比手写字符串稳，但库里没有现成的
-- 期望值，这里退一步按 `content` 缺失来识别——旧种子的特征就是没有它。

update msg_channel
set payload_template = '{"eventCode":{eventCode:json},"title":{title:json},"content":{content:json},"ext":{ext:json},"firstReceiveMs":{firstReceiveMs:default(null)},"businessTriggerMs":{businessTriggerMs:default(null)}}',
    updater_id = 0,
    update_ms = (extract(epoch from now()) * 1000)::int8
where code = 'websocket'
  and payload_template is not null
  and payload_template not like '%{content:json}%';
