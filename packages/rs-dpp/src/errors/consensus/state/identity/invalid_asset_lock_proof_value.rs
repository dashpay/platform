use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use crate::errors::consensus::state::state_error::StateError;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Asset lock value {value:?} must be higher than minimal value of {min_value:?}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidAssetLockProofValueError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub value: u64,
    pub min_value: u64,
}

impl InvalidAssetLockProofValueError {
    pub fn new(value: u64, min_value: u64) -> Self {
        Self { value, min_value }
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn min_value(&self) -> u64 {
        self.min_value
    }
}

impl From<InvalidAssetLockProofValueError> for ConsensusError {
    fn from(err: InvalidAssetLockProofValueError) -> Self {
        Self::StateError(StateError::InvalidAssetLockProofValueError(err))
    }
}
