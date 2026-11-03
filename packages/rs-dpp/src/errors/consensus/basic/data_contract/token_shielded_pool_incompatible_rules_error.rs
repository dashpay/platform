use crate::consensus::basic::BasicError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A token with a shielded pool cannot promise freezing or confiscation: shielded notes belong
/// to no identity account, so `freezeRules`, `unfreezeRules` and `destroyFrozenFundsRules` must
/// authorize no one (as change taker and as admin) when `hasShieldedPool` is set.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Token at position {token_contract_position} has a shielded pool, so {rule} must authorize no one to act and have no admin action takers: shielded notes cannot be frozen or destroyed"
)]
#[platform_serialize(unversioned)]
pub struct TokenShieldedPoolIncompatibleRulesError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_contract_position: TokenContractPosition,
    rule: String,
}

impl TokenShieldedPoolIncompatibleRulesError {
    pub fn new(token_contract_position: TokenContractPosition, rule: String) -> Self {
        Self {
            token_contract_position,
            rule,
        }
    }

    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }

    pub fn rule(&self) -> &str {
        &self.rule
    }
}

impl From<TokenShieldedPoolIncompatibleRulesError> for ConsensusError {
    fn from(err: TokenShieldedPoolIncompatibleRulesError) -> Self {
        Self::BasicError(BasicError::TokenShieldedPoolIncompatibleRulesError(err))
    }
}
