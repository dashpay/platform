use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Disabling a key with id {key_id:?} that is being added in same state transition")]
#[platform_serialize(unversioned)]
pub struct DisablingKeyIdAlsoBeingAddedInSameTransitionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_id: KeyID,
}

impl DisablingKeyIdAlsoBeingAddedInSameTransitionError {
    pub fn new(key_id: KeyID) -> Self {
        Self { key_id }
    }

    pub fn key_id(&self) -> KeyID {
        self.key_id
    }
}
impl From<DisablingKeyIdAlsoBeingAddedInSameTransitionError> for ConsensusError {
    fn from(err: DisablingKeyIdAlsoBeingAddedInSameTransitionError) -> Self {
        Self::BasicError(BasicError::DisablingKeyIdAlsoBeingAddedInSameTransitionError(err))
    }
}
