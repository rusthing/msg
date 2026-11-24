-- 位置帧上报时间字段改名：triggerTimestamp -> triggerMs
--
-- 前端实时地图的「更新多久前」改按设备上报时间（triggerMs）计算，与后端
-- LocatorPositionCache、报警中心口径统一。已有库的位置消息 ext 模板还写着
-- 旧键，这里直接改名，不保留 triggerTimestamp（消费方已全部切换）。
--
-- 覆盖两种存量模板：
--   1. 只有旧键 "...triggerTimestamp":{{triggerTimestamp}}..."  -> 换成 triggerMs；
--   2. 过渡版双写 "...triggerMs":{{triggerMs}},"triggerTimestamp":{{triggerTimestamp}}..."
--      -> 删掉旧键那一段。
--
-- 幂等：执行后两种旧形态都不存在，重复执行 where 不再命中。
-- 注：dtw 启动时 register_message 遇到已存在的消息只同步 persisted，不会用
-- 新模板覆盖库里的 ext_template，所以这条迁移是存量库生效的唯一途径。

update public.msg_message
set ext_template = case
        when ext_template like '%"triggerMs":{{triggerMs}}%'
             and ext_template like '%"triggerTimestamp":{{triggerTimestamp}}%'
            then replace(ext_template, ',"triggerTimestamp":{{triggerTimestamp}}', '')
        else replace(
            ext_template,
            '"triggerTimestamp":{{triggerTimestamp}}',
            '"triggerMs":{{triggerMs}}'
        )
    end,
    update_ms = 1795497600000
where event_code = 'telemetry.dtw.position.updated'
  and ext_template like '%"triggerTimestamp":{{triggerTimestamp}}%';
