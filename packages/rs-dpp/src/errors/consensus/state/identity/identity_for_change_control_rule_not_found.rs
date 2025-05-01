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
#[error("Identity {identity_id} required by change control rule \"{rule_path}\" in token position {token_position} of contract {contract_id} does not exist")]
#[platform_serialize(unversioned)]
pub struct IdentityForChangeControlRuleNotFoundError {
    contract_id: Identifier,
    token_position: u16,
    rule_path: String,
    identity_id: Identifier,
}

impl IdentityForChangeControlRuleNotFoundError {
    pub fn new(
        contract_id: Identifier,
        token_position: u16,
        rule_path: String,
        identity_id: Identifier,
    ) -> Self {
        Self {
            contract_id,
            token_position,
            rule_path,
            identity_id,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn token_position(&self) -> u16 {
        self.token_position
    }

    pub fn rule_path(&self) -> &str {
        &self.rule_path
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }
}

impl From<IdentityForChangeControlRuleNotFoundError> for ConsensusError {
    fn from(err: IdentityForChangeControlRuleNotFoundError) -> Self {
        Self::StateError(StateError::IdentityForChangeControlRuleNotFoundError(err))
    }
}
