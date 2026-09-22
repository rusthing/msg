#[cfg(feature = "server")]
use sea_orm::{ColIdx, DbErr, QueryResult, TryGetError, TryGetable};
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    AsRefStr,
    Display,
    EnumString,
    Serialize,
    Deserialize,
    ToSchema,
)]
#[strum(serialize_all = "kebab-case")]
pub enum DeliverChannelStatus {
    /// 投递中
    Delivering = 0,
    /// 投递成功
    Success = 1,
    /// 投递失败
    Failed = 2,
}

impl Default for DeliverChannelStatus {
    fn default() -> Self {
        Self::Delivering
    }
}

impl DeliverChannelStatus {
    pub const fn value(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(DeliverChannelStatus::Delivering),
            1 => Some(DeliverChannelStatus::Success),
            2 => Some(DeliverChannelStatus::Failed),
            _ => None,
        }
    }
}

impl From<DeliverChannelStatus> for i16 {
    fn from(v: DeliverChannelStatus) -> Self {
        v as i16
    }
}

impl From<i16> for DeliverChannelStatus {
    fn from(v: i16) -> Self {
        Self::from_i16(v).unwrap_or_default()
    }
}

#[cfg(feature = "server")]
impl TryGetable for DeliverChannelStatus {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        let value = <i16 as TryGetable>::try_get_by(res, idx)?;
        Self::from_i16(value).ok_or_else(|| {
            TryGetError::DbErr(DbErr::Custom(format!(
                "invalid deliver channel status value: {value}"
            )))
        })
    }
}