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
#[error("Token transfer recipient identity {recipient_id} doesn't exist")]
#[platform_serialize(unversioned)]
pub struct TokenTransferRecipientIdentityNotExistError {
    recipient_id: Identifier,
}

impl TokenTransferRecipientIdentityNotExistError {
    pub fn new(recipient_id: Identifier) -> Self {
        Self { recipient_id }
    }

    pub fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }
}

impl From<TokenTransferRecipientIdentityNotExistError> for ConsensusError {
    fn from(err: TokenTransferRecipientIdentityNotExistError) -> Self {
        Self::StateError(StateError::TokenTransferRecipientIdentityNotExistError(err))
    }
}
