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
#[error("Recipient identity {} does not exist", recipient_id)]
#[platform_serialize(unversioned)]
pub struct RecipientIdentityDoesNotExistError {
    recipient_id: Identifier,
}

impl RecipientIdentityDoesNotExistError {
    pub fn new(recipient_id: Identifier) -> Self {
        Self { recipient_id }
    }
    pub fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }
}

impl From<RecipientIdentityDoesNotExistError> for ConsensusError {
    fn from(err: RecipientIdentityDoesNotExistError) -> Self {
        Self::StateError(StateError::RecipientIdentityDoesNotExistError(err))
    }
}
