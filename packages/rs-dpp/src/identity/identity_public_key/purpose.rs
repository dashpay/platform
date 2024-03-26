use crate::identity::Purpose::{AUTHENTICATION, DECRYPTION, ENCRYPTION, SYSTEM, TRANSFER, VOTING};
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
    Ord,
    PartialOrd,
    Encode,
    Decode,
    Default,
    strum::EnumIter,
)]
pub enum Purpose {
    /// at least one authentication key must be registered for all security levels
    #[default]
    AUTHENTICATION = 0,
    /// this key cannot be used for signing documents
    ENCRYPTION = 1,
    /// this key cannot be used for signing documents
    DECRYPTION = 2,
    /// this key is used to sign credit transfer and withdrawal state transitions
    TRANSFER = 3,
    /// this key cannot be used for signing documents
    SYSTEM = 4,
    /// this key cannot be used for signing documents
    VOTING = 5,
}

impl From<Purpose> for [u8; 1] {
    fn from(purpose: Purpose) -> Self {
        [purpose as u8]
    }
}

impl From<Purpose> for &'static [u8; 1] {
    fn from(purpose: Purpose) -> Self {
        match purpose {
            AUTHENTICATION => &[0],
            ENCRYPTION => &[1],
            DECRYPTION => &[2],
            TRANSFER => &[3],
            SYSTEM => &[4],
            VOTING => &[5],
        }
    }
}

impl TryFrom<u8> for Purpose {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(AUTHENTICATION),
            1 => Ok(ENCRYPTION),
            2 => Ok(DECRYPTION),
            3 => Ok(TRANSFER),
            4 => Ok(SYSTEM),
            5 => Ok(VOTING),
            value => bail!("unrecognized purpose: {}", value),
        }
    }
}

#[cfg(feature = "cbor")]
impl Into<CborValue> for Purpose {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
impl std::fmt::Display for Purpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Purpose {
    /// The full range of purposes
    pub fn full_range() -> [Purpose; 4] {
        [AUTHENTICATION, ENCRYPTION, DECRYPTION, TRANSFER]
    }
    /// Just the authentication and withdraw purposes
    pub fn authentication_and_transfer() -> [Purpose; 2] {
        [AUTHENTICATION, TRANSFER]
    }
    /// Just the encryption and decryption purposes
    pub fn encryption_decryption() -> [Purpose; 2] {
        [ENCRYPTION, DECRYPTION]
    }
    /// The last purpose
    pub fn last() -> Purpose {
        Self::TRANSFER
    }
}
