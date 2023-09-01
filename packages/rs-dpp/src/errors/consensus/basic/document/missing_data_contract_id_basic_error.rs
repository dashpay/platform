use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Default,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error("$dataContractId is not present")]
#[platform_serialize(unversioned)]
pub struct MissingDataContractIdBasicError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDataContractIdBasicError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingDataContractIdBasicError> for ConsensusError {
    fn from(err: MissingDataContractIdBasicError) -> Self {
        Self::BasicError(BasicError::MissingDataContractIdBasicError(err))
    }
}
