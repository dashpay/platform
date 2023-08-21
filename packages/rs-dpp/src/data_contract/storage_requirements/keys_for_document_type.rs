use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde_repr::*;
use std::convert::TryFrom;

/// The Storage Key requirements
// @append_only
#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Copy, Clone, Encode, Decode)]
pub enum StorageKeyRequirements {
    Unique = 0,
    UniqueReplaceable = 1,
    Multiple = 2,
}

impl TryFrom<u8> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::UniqueReplaceable),
            2 => Ok(Self::Multiple),
            value => Err(ProtocolError::UnknownStorageKeyRequirements(format!(
                "unrecognized storage key requirements: {}",
                value
            ))),
        }
    }
}

impl TryFrom<i128> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::UniqueReplaceable),
            2 => Ok(Self::Multiple),
            value => Err(ProtocolError::UnknownStorageKeyRequirements(format!(
                "unrecognized storage key requirements: {}",
                value
            ))),
        }
    }
}
