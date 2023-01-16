use super::purpose::Purpose;
use anyhow::bail;
use ciborium::value::Value as CborValue;
use lazy_static::lazy_static;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::convert::TryFrom;

#[repr(u8)]
#[derive(
    Debug, PartialEq, Eq, Clone, Copy, Hash, Serialize_repr, Deserialize_repr, PartialOrd, Ord,
)]
pub enum SecurityLevel {
    MASTER = 0,
    CRITICAL = 1,
    HIGH = 2,
    MEDIUM = 3,
}

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
        write!(f, "{:?}", self)
    }
}

lazy_static! {
    pub static ref ALLOWED_SECURITY_LEVELS: HashMap<Purpose, Vec<SecurityLevel>> = {
        let mut m = HashMap::new();
        m.insert(
            Purpose::AUTHENTICATION,
            vec![
                SecurityLevel::MASTER,
                SecurityLevel::CRITICAL,
                SecurityLevel::HIGH,
                SecurityLevel::MEDIUM,
            ],
        );
        m.insert(Purpose::ENCRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::DECRYPTION, vec![SecurityLevel::MEDIUM]);
        m.insert(Purpose::WITHDRAW, vec![SecurityLevel::CRITICAL]);
        m
    };
}
