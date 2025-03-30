use crate::balances::credits::TokenAmount;
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
    "Identity has not agreed to pay the required token amount for token {}: required {}, min offer {:?}, max offer {:?}, action: {}",
    token_id,
    required_amount,
    identity_min_offer,
    identity_max_offer,
    action
)]
#[platform_serialize(unversioned)]
pub struct IdentityHasNotAgreedToPayRequiredTokenAmountError {
    token_id: Identifier,
    required_amount: u64,
    identity_min_offer: Option<TokenAmount>,
    identity_max_offer: Option<TokenAmount>,
    action: String,
}

impl IdentityHasNotAgreedToPayRequiredTokenAmountError {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        token_id: Identifier,
        required_amount: u64,
        identity_min_offer: Option<TokenAmount>,
        identity_max_offer: Option<TokenAmount>,
        action: String,
    ) -> Self {
        Self {
            token_id,
            required_amount,
            identity_min_offer,
            identity_max_offer,
            action,
        }
    }

    pub fn token_id(&self) -> &Identifier {
        &self.token_id
    }

    pub fn required_amount(&self) -> u64 {
        self.required_amount
    }

    pub fn identity_min_offer(&self) -> Option<TokenAmount> {
        self.identity_min_offer
    }

    pub fn identity_max_offer(&self) -> Option<TokenAmount> {
        self.identity_max_offer
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<IdentityHasNotAgreedToPayRequiredTokenAmountError> for ConsensusError {
    fn from(err: IdentityHasNotAgreedToPayRequiredTokenAmountError) -> Self {
        Self::StateError(StateError::IdentityHasNotAgreedToPayRequiredTokenAmountError(err))
    }
}
