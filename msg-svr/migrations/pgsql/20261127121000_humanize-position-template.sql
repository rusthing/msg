-- 位置更新消息的模板去掉设备卡号：标题和正文都改用「谁」
--
-- `telemetry.dtw.position.updated` 是消息中心里唯一渲染文案的定位消息
-- （报警类消息的三份模板留空，文案由报警策略渲染）。它的标题 / 正文是给人看
-- 的：大屏通知、定位卡以外的渠道转发都会直接用它。原来的默认文案是
--
--   标题：定位卡 {{locatorCode}} 位置已更新
--   正文：定位卡 {{locatorCode}} 位于 {{floor:default(未知)}} 层
--
-- `locatorCode` 是设备卡号，人要看的是「谁在这里」。位置消息的
-- annotations 里已经有 `bindTargetName`（绑定的人员姓名 / 车牌号，见 dtw
-- `position_updated_msg`），直接换掉卡号；未绑定时兜底「未知目标」。
--
-- 只替换默认片段，不动 `ext_template`：那是给大屏的机器载荷，`locatorCode`、
-- `triggerMs`、`regionId`、Unity 坐标都必须保留——大屏按卡号做去重与落点，
-- 原始时间戳用于算「这次定位多久之前」。运营改过的文案也原样保留。
--
-- 不按 `event_code` 定位：存量库还有旧编码 `dtw.position.updated`（基线
-- 20261109000002 与 dtw 现行登记用的是 `telemetry.dtw.position.updated`），
-- 按模板片段匹配两种编码都能覆盖，也不会误改到别的消息。
--
-- 幂等：替换后旧片段不再出现，重复执行不再改动。最新部署的 dtw 代码常量
-- （`POSITION_TITLE_TEMPLATE_DEFAULT` / `POSITION_CONTENT_TEMPLATE_DEFAULT`）
-- 已是新文案；存量库由本迁移补齐，两边结果一致。

update public.msg_message
set
    title_template = replace(
        title_template,
        '定位卡 {{locatorCode}} 位置已更新',
        '{{bindTargetName:default(未知目标)}} 位置已更新'
    ),
    content_template = replace(
        content_template,
        '定位卡 {{locatorCode}} 位于',
        '{{bindTargetName:default(未知目标)}} 位于'
    ),
    update_ms = 1795752000000
where (
    title_template like '%定位卡 {{locatorCode}} 位置已更新%'
    or content_template like '%定位卡 {{locatorCode}} 位于%'
);
