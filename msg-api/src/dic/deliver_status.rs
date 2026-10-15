use robotech::macros::dic;
use strum::{AsRefStr, Display, EnumString};

/// 被投递相关 VO 用作字段，所以必须声明 `default`。
#[dic(default = Delivering)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr, Display, EnumString)]
#[strum(serialize_all = "kebab-case")]
pub enum DeliverStatus {
    /// 投递中
    Delivering = 0,
    /// 投递部分成功
    PartialSuccess = 1,
    /// 投递全部成功
    AllSuccess = 2,
    /// 投递全部失败
    AllFailed = 3,
}
