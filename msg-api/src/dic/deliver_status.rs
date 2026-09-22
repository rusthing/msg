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

impl Default for DeliverStatus {
    fn default() -> Self {
        Self::Delivering
    }
}

impl DeliverStatus {
    pub const fn value(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(DeliverStatus::Delivering),
            1 => Some(DeliverStatus::PartialSuccess),
            2 => Some(DeliverStatus::AllSuccess),
            3 => Some(DeliverStatus::AllFailed),
            _ => None,
        }
    }
}

impl From<DeliverStatus> for i16 {
    fn from(v: DeliverStatus) -> Self {
        v as i16
    }
}

impl From<i16> for DeliverStatus {
    fn from(v: i16) -> Self {
        Self::from_i16(v).unwrap_or_default()
    }
}

#[cfg(feature = "server")]
impl TryGetable for DeliverStatus {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        let value = <i16 as TryGetable>::try_get_by(res, idx)?;
        Self::from_i16(value).ok_or_else(|| {
            TryGetError::DbErr(DbErr::Custom(format!("invalid deliver status value: {value}")))
        })
    }
}