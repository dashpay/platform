use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("The specified new authorized action taker identity {identity_id} does not exist")]
#[platform_serialize(unversioned)]
pub struct NewAuthorizedActionTakerIdentityDoesNotExistError {
    identity_id: Identifier,
}

impl NewAuthorizedActionTakerIdentityDoesNotExistError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

impl From<NewAuthorizedActionTakerIdentityDoesNotExistError> for ConsensusError {
    fn from(err: NewAuthorizedActionTakerIdentityDoesNotExistError) -> Self {
        Self::StateError(StateError::NewAuthorizedActionTakerIdentityDoesNotExistError(err))
    }
}
