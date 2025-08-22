use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Unrecognized gas fees paid by mode: allowed {:?}, got {}",
    allowed_values,
    received
)]
#[platform_serialize(unversioned)]
pub struct UnknownGasFeesPaidByError {
    /*
    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING A NEW VERSION
    */
    allowed_values: Vec<u8>,
    received: u64,
}

impl UnknownGasFeesPaidByError {
    pub fn new(allowed_values: Vec<u8>, received: u64) -> Self {
        Self {
            allowed_values,
            received,
        }
    }

    pub fn allowed_values(&self) -> Vec<u8> {
        self.allowed_values.clone()
    }

    pub fn received(&self) -> u64 {
        self.received
    }
}

impl From<UnknownGasFeesPaidByError> for ConsensusError {
    fn from(err: UnknownGasFeesPaidByError) -> Self {
        Self::BasicError(BasicError::UnknownGasFeesPaidByError(err))
    }
}
