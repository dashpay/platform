use anyhow::bail;
use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;

use serde_repr::{Deserialize_repr, Serialize_repr};

use std::convert::TryFrom;

#[repr(u8)]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Hash,
    Serialize_repr,
    Deserialize_repr,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    Default,
    strum::EnumIter,
)]
#[ferment_macro::export]
pub enum SecurityLevel {
    #[default]
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for SecurityLevel {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}

impl TryFrom<u8> for SecurityLevel {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::MASTER),
            1 => Ok(Self::CRITICAL),
            2 => Ok(Self::HIGH),
            3 => Ok(Self::MEDIUM),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl SecurityLevel {
    /// The full range of security levels
    pub fn full_range() -> [SecurityLevel; 4] {
        [Self::MASTER, Self::CRITICAL, Self::HIGH, Self::MEDIUM]
    }
    pub fn last() -> SecurityLevel {
        Self::MEDIUM
    }
    pub fn lowest_level() -> SecurityLevel {
        Self::MEDIUM
    }
    pub fn highest_level() -> SecurityLevel {
        Self::MASTER
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
