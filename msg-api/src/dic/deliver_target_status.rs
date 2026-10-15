use robotech::macros::dic;
use strum::{AsRefStr, Display, EnumString};

/// 被投递目标 VO 用作字段，所以必须声明 `default`。
#[dic(default = Delivering)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum DeliverTargetStatus {
    /// 投递中
    Delivering = 0,
    /// 投递成功
    Success = 1,
    /// 投递失败
    Failed = 2,
    /// 目标已读
    Read = 3,
}
