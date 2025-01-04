use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::change_control_rules::authorized_action_takers::AuthorizedActionTakers;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Identity {} is not authorized to perform action: {}. Authorized action takers: {:?}",
    identity_id,
    action,
    authorized_action_takers
)]
#[platform_serialize(unversioned)]
pub struct UnauthorizedTokenActionError {
    identity_id: Identifier,
    action: String,
    authorized_action_takers: AuthorizedActionTakers,
}

impl UnauthorizedTokenActionError {
    pub fn new(
        identity_id: Identifier,
        action: String,
        authorized_action_takers: AuthorizedActionTakers,
    ) -> Self {
        Self {
            identity_id,
            action,
            authorized_action_takers,
        }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }

    pub fn action(&self) -> &str {
        &self.action
    }

    pub fn authorized_action_takers(&self) -> &AuthorizedActionTakers {
        &self.authorized_action_takers
    }
}

impl From<UnauthorizedTokenActionError> for ConsensusError {
    fn from(err: UnauthorizedTokenActionError) -> Self {
        Self::StateError(StateError::UnauthorizedTokenActionError(err))
    }
}
