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
#[error(
    "Identity {} account is not frozen for token {}. Action attempted: {}",
    identity_id,
    token_id,
    action
)]
#[platform_serialize(unversioned)]
pub struct IdentityTokenAccountNotFrozenError {
    token_id: Identifier,
    identity_id: Identifier,
    action: String,
}

impl IdentityTokenAccountNotFrozenError {
    pub fn new(token_id: Identifier, identity_id: Identifier, action: String) -> Self {
        Self {
            token_id,
            identity_id,
            action,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<IdentityTokenAccountNotFrozenError> for ConsensusError {
    fn from(err: IdentityTokenAccountNotFrozenError) -> Self {
        Self::StateError(StateError::IdentityTokenAccountNotFrozenError(err))
    }
}
