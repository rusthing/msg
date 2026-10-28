-- 把「大屏」（code = screen）目标类别随库种好
--
-- 初始脚本（20261026000001_init-data.sql）里故意没种它：大屏是动态接入的自定义
-- 类别，谁先用到谁登记。真跑起来发现这条路走不通，问题出在启动顺序上：
--
--   1. 发布方登记类别是「先查后插」，两个发布方（dtw、其它大屏消息的发布方）
--      同时起来就会并发插入，一个成功一个撞唯一键，只能靠回查兜底；
--   2. 类别建好了、渠道绑定还没建，中间这段时间投递任务查不到渠道，
--      消息落库但不投递——这个窗口没有任何报错，只是屏上不显示；
--   3. 发布方先于消息中心起来时整段跳过登记，要等下一次重启才补上。
--
-- 大屏不是哪一家的私有类别，它是平台主线目标（报警、定位、门禁都往这块屏上发），
-- 和 user / role / group 是同一层级的部署级字典。所以这里把它挪到随库种好：
-- 渠道绑定一步到位，发布方只剩下「按编码查 ID 来挂消息目标」这一件只读的事，
-- 不再有插入竞争，也没有「类别在、绑定不在」的空窗。
--
-- 发布方侧不用改：它们本来就是「按 code 查，查到就复用」，库里已经有了就直接命中，
-- 之前已经在 dtw 启动时建过这条类别的库，这里的 `on conflict do nothing` 会让它原样保留。
--
-- 审计字段的 0 是 SYS_OPERATOR_USER_ID（系统操作员），时间戳取执行时刻。

insert into msg_target_category (
   id, code, name, remark, creater_id, create_ms, updater_id, update_ms
) values (
   4,
   'screen',
   '大屏',
   '自定义目标：大屏长连接',
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
)
on conflict do nothing;

-- 大屏这一类只走 websocket：投递任务是「目标 × 渠道」展开出来的，没有这条绑定，
-- 挂在大屏类别上的消息（dtw 的位置更新、报警中心的 SOS）一条都发不出去。
--
-- 类别 ID 和渠道 ID 都按编码现查，不写死：类别可能已经在 dtw 启动时用雪花 ID 建过，
-- 渠道 ID 也可能因部署而异（初始脚本种的是 2，别处可能是别的值）。
insert into msg_target_category_channel (
   id, target_category_id, channel_id, creater_id, create_ms, updater_id, update_ms
)
select
   1,
   tc.id,
   c.id,
   0,
   (extract(epoch from now()) * 1000)::int8,
   0,
   (extract(epoch from now()) * 1000)::int8
from msg_target_category tc
join msg_channel c on c.code = 'websocket'
where tc.code = 'screen'
  and not exists (
      select 1
      from msg_target_category_channel b
      where b.target_category_id = tc.id
        and b.channel_id = c.id
  )
on conflict do nothing;
