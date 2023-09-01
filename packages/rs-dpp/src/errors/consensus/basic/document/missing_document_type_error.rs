use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Default,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserialize,
)]
#[error("$type is not present")]
#[platform_serialize(unversioned)]
pub struct MissingDocumentTypeError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDocumentTypeError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingDocumentTypeError> for ConsensusError {
    fn from(err: MissingDocumentTypeError) -> Self {
        Self::BasicError(BasicError::MissingDocumentTypeError(err))
    }
}
