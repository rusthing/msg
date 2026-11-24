-- 邮件渠道 options 对齐代码：去掉无效的 ssl，默认端口改 25
--
-- 基线迁移里邮件渠道写的是：
--   {"smtpHost":"","smtpPort":465,"username":"","password":"","fromAddress":"","ssl":true}
-- 但 EmailOptions 没有 ssl 字段（只有 requireTls），反序列化时被静默忽略，端口却停在 465。
-- 465 是隐式 TLS，当前发送器只支持明文 SMTP，配上去只会超时；文档里也写明不支持
-- STARTTLS/465。所以：
--   - 默认端口改成 25（内网中继最常见的明文端口）；
--   - 删掉无效的 ssl，用 requireTls=false 表达「不加密」；
--   - 补上代码里有默认值、但配置里看不到的字段，运营在控制台里能直接改。
--
-- 只改与基线完全一致的那份配置：已经按实际网关改过 options 的库不动，避免覆盖
-- 现场参数（这些库里如果还留着 ssl，也本来就不生效）。

update public.msg_channel
set options = '{"smtpHost":"","smtpPort":25,"username":"","password":"","fromAddress":"","fromName":"","contentType":"text/plain","subjectTemplate":"","bodyTemplate":"","timeoutMs":10000,"requireTls":false}'
where code = 'email'
  and options = '{"smtpHost":"","smtpPort":465,"username":"","password":"","fromAddress":"","ssl":true}';
