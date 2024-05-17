use bincode::{Decode, Encode};
#[cfg(feature = "cbor")]
use ciborium::value::Value as CborValue;

use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::consensus::basic::data_contract::UnknownSecurityLevelError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
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
    MASTER = 0,
    CRITICAL = 1,
    #[default]
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
    type Error = ProtocolError;
    fn try_from(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0 => Ok(Self::MASTER),
            1 => Ok(Self::CRITICAL),
            2 => Ok(Self::HIGH),
            3 => Ok(Self::MEDIUM),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownSecurityLevelError(
                    UnknownSecurityLevelError::new(vec![0, 1, 2, 3], value),
                ))
                .into(),
            )),
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
    pub fn stronger_security_than(self: SecurityLevel, rhs: SecurityLevel) -> bool {
        // Example:
        // self: High 2 rhs: Master 0
        // Master has a stronger security level than high
        // We expect False
        // High < Master
        // 2 < 0 <=> false
        (self as u8) < (rhs as u8)
    }

    pub fn stronger_or_equal_security_than(self: SecurityLevel, rhs: SecurityLevel) -> bool {
        (self as u8) <= (rhs as u8)
    }
}

impl std::fmt::Display for SecurityLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
