use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize)]
#[platform_serialize(unversioned)]
pub enum TokenConfigurationIdentityContext {
    ChangeControlRule(String),
    DefaultMintingRecipient,
    PerpetualDistributionRecipient,
    PreProgrammedDistributionRecipient,
}

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity {identity_id} required in token position {token_position} of contract {contract_id} for {context:?} does not exist")]
#[platform_serialize(unversioned)]
pub struct IdentityInTokenConfigurationNotFoundError {
    contract_id: Identifier,
    token_position: u16,
    context: TokenConfigurationIdentityContext,
    identity_id: Identifier,
}

impl IdentityInTokenConfigurationNotFoundError {
    pub fn new(
        contract_id: Identifier,
        token_position: u16,
        context: TokenConfigurationIdentityContext,
        identity_id: Identifier,
    ) -> Self {
        Self {
            contract_id,
            token_position,
            context,
            identity_id,
        }
    }

    pub fn contract_id(&self) -> &Identifier {
        &self.contract_id
    }

    pub fn token_position(&self) -> u16 {
        self.token_position
    }

    pub fn context(&self) -> &TokenConfigurationIdentityContext {
        &self.context
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

impl From<IdentityInTokenConfigurationNotFoundError> for ConsensusError {
    fn from(err: IdentityInTokenConfigurationNotFoundError) -> Self {
        Self::StateError(StateError::IdentityInTokenConfigurationNotFoundError(err))
    }
}
