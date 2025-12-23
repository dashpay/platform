use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Output address cannot also be an input address")]
#[platform_serialize(unversioned)]
pub struct OutputAddressAlsoInputError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
}

impl OutputAddressAlsoInputError {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for OutputAddressAlsoInputError {
    fn default() -> Self {
        Self::new()
    }
}

impl From<OutputAddressAlsoInputError> for ConsensusError {
    fn from(err: OutputAddressAlsoInputError) -> Self {
        Self::BasicError(BasicError::OutputAddressAlsoInputError(err))
    }
}
