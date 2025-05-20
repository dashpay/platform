use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity to freeze {} does not exist", identity_to_freeze_id)]
#[platform_serialize(unversioned)]
pub struct IdentityToFreezeDoesNotExistError {
    identity_to_freeze_id: Identifier,
}

impl IdentityToFreezeDoesNotExistError {
    pub fn new(identity_to_freeze_id: Identifier) -> Self {
        Self {
            identity_to_freeze_id,
        }
    }
    pub fn identity_to_freeze_id(&self) -> Identifier {
        self.identity_to_freeze_id
    }
}

impl From<IdentityToFreezeDoesNotExistError> for ConsensusError {
    fn from(err: IdentityToFreezeDoesNotExistError) -> Self {
        Self::StateError(StateError::IdentityToFreezeDoesNotExistError(err))
    }
}
