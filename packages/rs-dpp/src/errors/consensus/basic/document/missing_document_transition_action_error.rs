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
#[error("$action is not present")]
#[platform_serialize(unversioned)]
pub struct MissingDocumentTransitionActionError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDocumentTransitionActionError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingDocumentTransitionActionError> for ConsensusError {
    fn from(err: MissingDocumentTransitionActionError) -> Self {
        Self::BasicError(BasicError::MissingDocumentTransitionActionError(err))
    }
}
