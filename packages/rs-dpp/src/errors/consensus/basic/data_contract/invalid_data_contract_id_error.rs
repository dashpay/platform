use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract Id must be {}, got {}", bs58::encode(expected_id).into_string(), bs58::encode(invalid_id).into_string())]
#[platform_serialize(unversioned)]
pub struct InvalidDataContractIdError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    expected_id: Vec<u8>,
    invalid_id: Vec<u8>,
}

impl InvalidDataContractIdError {
    pub fn new(expected_id: Vec<u8>, invalid_id: Vec<u8>) -> Self {
        Self {
            expected_id,
            invalid_id,
        }
    }

    pub fn expected_id(&self) -> Vec<u8> {
        self.expected_id.clone()
    }
    pub fn invalid_id(&self) -> Vec<u8> {
        self.invalid_id.clone()
    }
}

impl From<InvalidDataContractIdError> for ConsensusError {
    fn from(err: InvalidDataContractIdError) -> Self {
        Self::BasicError(BasicError::InvalidDataContractIdError(err))
    }
}
