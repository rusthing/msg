-- 模板语法迁移：旧单花括号 `{key}` → 新双花括号 `{{key}}`
--
-- 背景：共享渲染 crate `robotech-template` 已彻底移除旧语法，模板里残留的
-- `{key}` 不会被替换。本迁移把库里已有的模板一次性转换，避免升级后文案里
-- 留下占位符字面量。
--
-- 转换规则：
--   {key}            → {{key}}
--   {key:json}       → {{key:json}}
--   {key:time(%F)}   → {{key:time(%F)}}
-- 只匹配「`{` + 变量名 + 可选 `:过滤器链` + `}`」；JSON 对象括号（`{}`、
-- `{"k":...}`）因为 `{` 后不是变量字符，不会被误伤。
--
-- 幂等：先把已经是 `{{` 的部分临时替换成控制字符 \x01，再转换单花括号，
-- 最后还原。重复执行时 `{{x}}` 内层不会再次被加括号。
--
-- 涉及列：
--   msg_message.title_template / content_template / ext_template
--   msg_channel.payload_template
--
-- 输出：迁移前后分别 RAISE NOTICE 出命中行数与残留行数，便于复核。

do $$
declare
  -- 旧语法检测式：花色括号前面不是 `{`（排除已经是双花括号的写法）
  re_old constant text := $re$(^|[^{])\{\w+(:[^{}]*)?\}$re$;
  -- 实际替换式：不限制前一个字符，交给外层保护逻辑处理
  re_old_all constant text := $re$\{(\w+(?::[^{}]*)?)\}$re$;

  v_message_before integer;
  v_channel_before integer;
  v_message_after integer;
  v_channel_after integer;
begin
  select count(*) into v_message_before
  from msg_message
  where title_template ~ re_old
     or content_template ~ re_old
     or ext_template ~ re_old;

  select count(*) into v_channel_before
  from msg_channel
  where payload_template ~ re_old;

  raise notice '模板语法迁移前命中：msg_message=% 行，msg_channel=% 行',
    v_message_before, v_channel_before;

  update msg_message
  set title_template = replace(
        regexp_replace(
          replace(title_template, '{{', E'\x01'),
          re_old_all, '{{\1}}', 'g'),
        E'\x01', '{{')
  where title_template ~ re_old;

  update msg_message
  set content_template = replace(
        regexp_replace(
          replace(content_template, '{{', E'\x01'),
          re_old_all, '{{\1}}', 'g'),
        E'\x01', '{{')
  where content_template ~ re_old;

  update msg_message
  set ext_template = replace(
        regexp_replace(
          replace(ext_template, '{{', E'\x01'),
          re_old_all, '{{\1}}', 'g'),
        E'\x01', '{{')
  where ext_template ~ re_old;

  update msg_channel
  set payload_template = replace(
        regexp_replace(
          replace(payload_template, '{{', E'\x01'),
          re_old_all, '{{\1}}', 'g'),
        E'\x01', '{{')
  where payload_template ~ re_old;

  select count(*) into v_message_after
  from msg_message
  where title_template ~ re_old
     or content_template ~ re_old
     or ext_template ~ re_old;

  select count(*) into v_channel_after
  from msg_channel
  where payload_template ~ re_old;

  raise notice '模板语法迁移后残留旧语法：msg_message=% 行，msg_channel=% 行',
    v_message_after, v_channel_after;
end $$;

-- 老迁移（20261031000002）里的列注释还是单花括号写法，已应用迁移不能改，
-- 在这里补一次注释，避免新环境看列注释时又照抄旧语法。
comment on column msg_channel.payload_template is
$c$载荷模板
渲染成这个渠道真正的报文正文，为空时用渠道代码里的默认结构（websocket 是 eventCode/title/content/ext/firstReceiveMs/businessTriggerMs）
变量只有服务端给的这几个：{{eventCode}} {{title}} {{content}} {{ext}} {{firstReceiveMs}} {{businessTriggerMs}}，语法同消息模板（{{变量:过滤器}}，字符串用 :json）
websocket 渠道要求渲染结果是合法 JSON，不是就告警并回退默认帧
@len 0..$c$;
