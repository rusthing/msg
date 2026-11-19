use robotech::macros::dao;

/// 用户投影（由同步写入，禁止在 msg 侧修改）
///
/// 只有主键，没有 AK：投影是副本，唯一性由源库保证。
#[dao]
pub struct MsgUserDao;
