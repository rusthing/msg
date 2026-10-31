-- 给渠道补载荷模板
--
-- 背景：websocket 渠道发给大屏的那一帧，结构一直写死在发送器里（`WsPush` 的
-- eventCode / title / content / ext / firstReceiveMs / businessTriggerMs 六个字段）。
-- 每换一次大屏协议就要改代码、发版本，而这几块屏分属不同项目，字段叫法各有各的习惯。
--
-- 改法：把「帧长什么样」挪到渠道配置上。`payload_template` 渲染出这条渠道真正的
-- 报文正文，发送器只负责把渲染结果写进连接：
--
--   msg_message.title_template / content_template / ext_template   → 渲染出文案与结构化数据
--   msg_channel.payload_template                                   → 决定这些结果在帧里怎么摆
--
-- 两层模板分开的原因：前者是「说什么」（一条消息一份，跟着事件编码走），后者是
-- 「怎么说话」（一个渠道一份，所有走这个渠道的消息共用）。短信、邮件将来接进来时
-- 各自的载荷格式不同，但文案不用重配。
--
-- 允许为空，且**空就是现在的行为**：为空时用渠道代码里的默认结构（websocket 就是
-- `WsPush` 那几个字段）。这样这次上线对已有部署零影响，运营想改结构时再填。
--
-- 模板变量只有服务端给的这几个：{eventCode} {title} {content} {ext}
-- {firstReceiveMs} {businessTriggerMs}，语法与消息模板完全一致（`{变量:过滤器}`，
-- 字符串要用 `:json` 转义成 JSON 字面量）。变量缺失时 `{ext:json}` / `{firstReceiveMs:json}`
-- 渲染成 null；裸写占位符会留下字面量，整帧判为非法 JSON 后回退默认帧。
--
-- websocket 渠道要求渲染结果仍是合法 JSON：模板写错（引号没转义、用了不存在的变量
-- 留下占位符）时发送器会告警并回退到默认帧，不会把坏帧推给大屏——大屏解析失败会
-- 丢掉整帧，比收到旧结构更糟。
--
-- 例：等价于默认帧；把内容原样嵌成对象、并且不要 ext 字段时也这么写：
--
--   {"eventCode":{eventCode:json},"title":{title:json},
--    "content":{content},"recv":{firstReceiveMs:json}}
--
-- 注意字符串变量必须带 `:json`：`{title}` 裸代入时标题里的引号会把整帧撑破，
-- 渲染结果不再是合法 JSON，发送器会回退默认帧（日志里有 warn）。

alter table msg_channel
   add column payload_template TEXT null;

comment on column msg_channel.payload_template is
'载荷模板
渲染成这个渠道真正的报文正文，为空时用渠道代码里的默认结构（websocket 是 eventCode/title/content/ext/firstReceiveMs/businessTriggerMs）
变量只有服务端给的这几个：{eventCode} {title} {content} {ext} {firstReceiveMs} {businessTriggerMs}，语法同消息模板（{变量:过滤器}，字符串用 :json）
websocket 渠道要求渲染结果是合法 JSON，不是就告警并回退默认帧
@len 0..';
