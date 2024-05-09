use crate::consensus::basic::data_contract::UnknownStorageKeyRequirementsError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use serde_repr::*;
use std::convert::TryFrom;

/// The Storage Key requirements
// @append_only
#[repr(u8)]
#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Eq, Copy, Clone, Encode, Decode)]
pub enum StorageKeyRequirements {
    Unique = 0,
    Multiple = 1,
    MultipleReferenceToLatest = 2,
}

impl TryFrom<u8> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::Multiple),
            2 => Ok(Self::MultipleReferenceToLatest),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownStorageKeyRequirementsError(
                    UnknownStorageKeyRequirementsError::new(vec![0, 1, 3], value.into()),
                ))
                .into(),
            )),
        }
    }
}

impl TryFrom<i128> for StorageKeyRequirements {
    type Error = ProtocolError;
    fn try_from(value: i128) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Unique),
            1 => Ok(Self::Multiple),
            2 => Ok(Self::MultipleReferenceToLatest),
            value => Err(ProtocolError::ConsensusError(
                ConsensusError::BasicError(BasicError::UnknownStorageKeyRequirementsError(
                    UnknownStorageKeyRequirementsError::new(vec![0, 1, 3], value),
                ))
                .into(),
            )),
        }
    }
}
