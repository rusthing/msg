-- 把 websocket 渠道的帧结构显式种进库里
--
-- 上一条迁移（20261031000002）只加了列，种子里的 websocket 渠道留空 → 发送器走
-- 默认帧。可默认帧定义在代码里，运营在库里看不到「大屏收到的到底长什么样」，
-- 想改协议得先读一遍 Rust。这里把它落成一条可读、可改的模板。
--
-- 只下发机器要看的三样东西：
--
--   eventCode          大屏按它分派渲染；
--   ext                结构化数据（报警中心给的 alertId / alertStatus / eventCode
--                      就在这一段里），没带 ext 的消息渲染成 null；
--   firstReceiveMs /
--   businessTriggerMs  两个时间锚点，缺了渲染成 null，大屏按「不可算」处理。
--
-- 标题与内容不下发：那是给人看的文案，由 msg_message 的三份模板渲染（一条消息
-- 一份），大屏按 eventCode 自己出词。谁要这两段，改这个模板加上
-- `"title":{title:json}` / `"content":{content}` 即可，不用改代码。
--
-- 语法与消息模板一致，两个容易写错的地方：
--
--   {eventCode:json}                字符串必须过 :json，否则没有引号，整帧非法；
--   {firstReceiveMs:default(null)}  数字要裸着输出才是数字。不能用 :json（会变成
--                                   字符串 "1761…"），也不能裸写占位符——缺值时
--                                   会留下 `{firstReceiveMs}` 字面量，整帧不是合法
--                                   JSON，发送器按回退规则改用默认帧，标题内容又
--                                   跟着漏下去。default(null) 两头都对：有值是数字，
--                                   没值是 JSON 的 null。
--
-- 只补空模板：运营已经按现场协议改过的，这里不覆盖。审计字段的 0 是
-- SYS_OPERATOR_USER_ID（系统操作员），时间戳取执行时刻。

update msg_channel
set payload_template = '{"eventCode":{eventCode:json},"ext":{ext:json},"firstReceiveMs":{firstReceiveMs:default(null)},"businessTriggerMs":{businessTriggerMs:default(null)}}',
    updater_id = 0,
    update_ms = (extract(epoch from now()) * 1000)::int8
where code = 'websocket'
  and payload_template is null;
