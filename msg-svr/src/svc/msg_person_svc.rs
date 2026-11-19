use robotech::macros::svc;

/// 人员投影（由同步写入，禁止在 msg 侧修改）
///
/// 生成的方法里 `sync` 正是同步入口：按 id 判存在，存在时只有
/// 「库里的 `update_ms` < 传入的 `update_ms`」才改写，乱序/重复重放被丢弃。
/// 不挂缓存刷新钩子：投影变化不影响消息缓存。
#[svc]
pub struct MsgPersonSvc;
