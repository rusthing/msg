-- websocket 大屏帧补齐时间锚点
--
-- 位置更新帧原先只有 data 里的 `triggerMs`（设备触发时间）：服务端接收时刻
-- `firstReceiveMs` 虽然在投递任务里一路带到了渠道发送器，但渠道模板没引用它，
-- 「msg 收到报文」与「推给大屏」两个时刻更没有落到帧上。大屏只能拿本地收帧
-- 时刻算延迟，设备时钟一偏就完全失真。
--
-- 渠道发送器现在会给模板注入五个变量：
--   {{firstTriggerMs}}     事件起点（发布方时钟，可空）
--   {{firstReceiveMs}}     链路第一跳收到原始事件的时刻（服务端时钟，可空）
--   {{receiveMs}}          消息中心收到报文的时刻（服务端时钟，取 create_ms）
--   {{businessTriggerMs}}  本次通知的业务时间（发布方时钟，可空）
--   {{sendMs}}             本渠道把这一帧写出去的时刻（服务端时钟）
-- 本迁移把种子里的 websocket 载荷模板换成带这些字段的版本，存量库由此生效；
-- 默认帧（渠道没配模板时）由代码一并补齐，不依赖本迁移。
--
-- 只替换基线种子那一份模板（按片段匹配，兼容首行 @engine 声明有无）。运营
-- 自定义过的渠道模板不覆盖：那种模板的变量名/结构由自定义方维护，需要照上面
-- 的清单手动补字段。
--
-- 幂等：替换后旧片段不再出现，重复执行不再改动。

update public.msg_channel
set payload_template = '@engine: robotech
{"eventCode":{{eventCode:json}},"data":{{ext:json}},"firstTriggerMs":{{firstTriggerMs:default(null)}},"firstReceiveMs":{{firstReceiveMs:default(null)}},"receiveMs":{{receiveMs:default(null)}},"businessTriggerMs":{{businessTriggerMs:default(null)}},"sendMs":{{sendMs:default(null)}}}',
    update_ms = 1795765500000
where code = 'websocket'
  and payload_template like '%"eventCode":{{eventCode:json}},"data":{{ext:json}}}%';
