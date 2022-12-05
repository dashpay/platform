use anyhow::bail;
use ciborium::value::Value as CborValue;
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::convert::TryFrom;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
pub enum KeyType {
    ECDSA_SECP256K1 = 0,
    BLS12_381 = 1,
    ECDSA_HASH160 = 2,
    BIP13_SCRIPT_HASH = 3,
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<u8> for KeyType {
    type Error = anyhow::Error;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ECDSA_SECP256K1),
            1 => Ok(Self::BLS12_381),
            2 => Ok(Self::ECDSA_HASH160),
            3 => Ok(Self::BIP13_SCRIPT_HASH),
            value => bail!("unrecognized security level: {}", value),
        }
    }
}

impl Into<CborValue> for KeyType {
    fn into(self) -> CborValue {
        CborValue::from(self as u128)
    }
}
