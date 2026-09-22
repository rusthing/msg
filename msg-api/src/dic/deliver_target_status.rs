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
pub enum DeliverTargetStatus {
    /// 投递中
    Delivering = 0,
    /// 投递成功
    Success = 1,
    /// 投递超时
    Timeout = 2,
    /// 目标已读
    Read = 3,
}

impl Default for DeliverTargetStatus {
    fn default() -> Self {
        Self::Delivering
    }
}

impl DeliverTargetStatus {
    pub const fn value(self) -> i16 {
        self as i16
    }

    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(DeliverTargetStatus::Delivering),
            1 => Some(DeliverTargetStatus::Success),
            2 => Some(DeliverTargetStatus::Timeout),
            3 => Some(DeliverTargetStatus::Read),
            _ => None,
        }
    }
}

impl From<DeliverTargetStatus> for i16 {
    fn from(v: DeliverTargetStatus) -> Self {
        v as i16
    }
}

impl From<i16> for DeliverTargetStatus {
    fn from(v: i16) -> Self {
        Self::from_i16(v).unwrap_or_default()
    }
}

#[cfg(feature = "server")]
impl TryGetable for DeliverTargetStatus {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        let value = <i16 as TryGetable>::try_get_by(res, idx)?;
        Self::from_i16(value).ok_or_else(|| {
            TryGetError::DbErr(DbErr::Custom(format!(
                "invalid deliver target status value: {value}"
            )))
        })
    }
}