-- 列注释口径收敛：模拟标记只认 simulated=true。
--
-- 20261109000000_create-table.sql 是当时 PD 导出脚本的逐字节副本，已经提交，
-- 改它会破坏 sqlx 的 checksum，因此保持原样不动。当前模型三件套
-- （msg.ldm / msg-pgsql.pdm / model/create-table-pgsql.sql）已同步成下面的新措辞，
-- 由本迁移把运行时库里的列注释刷到同一口径：
-- 新建库 = create-table 基线 + 后续 alter-table 增量 = 当前模型导出脚本。
--
-- 只改注释，不动任何列定义与数据；重复执行结果相同。

comment on column msg_dead_letter.simulated is
'是否模拟：死信落库时按原始负载的 simulated=true 标记写入';

comment on column msg_delivery.simulated is
'是否模拟：发布方带 simulated=true 标记时为是';

comment on column msg_message.sim_enabled is
'是否允许模拟通知投递
默认 false；关闭时带 simulated=true 标记的通知会被丢弃';
