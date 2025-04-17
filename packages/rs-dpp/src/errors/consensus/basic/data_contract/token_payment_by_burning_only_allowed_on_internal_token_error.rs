use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::data_contract::TokenContractPosition;
use crate::tokens::calculate_token_id;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use std::fmt;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
pub struct TokenPaymentByBurningOnlyAllowedOnInternalTokenError {
    external_token_contract_id: Identifier,
    external_token_contract_token_position: TokenContractPosition,
    action: String,
}

impl TokenPaymentByBurningOnlyAllowedOnInternalTokenError {
    pub fn new(
        external_token_contract_id: Identifier,
        external_token_contract_token_position: TokenContractPosition,
        action: String,
    ) -> Self {
        Self {
            external_token_contract_id,
            external_token_contract_token_position,
            action,
        }
    }

    pub fn external_token_contract_id(&self) -> Identifier {
        self.external_token_contract_id
    }

    pub fn external_token_id(&self) -> Identifier {
        calculate_token_id(
            self.external_token_contract_id.as_bytes(),
            self.external_token_contract_token_position,
        )
        .into()
    }

    pub fn external_token_contract_token_position(&self) -> TokenContractPosition {
        self.external_token_contract_token_position
    }

    pub fn action(&self) -> &str {
        self.action.as_str()
    }
}

// Custom Display implementation to include external_token_id
impl fmt::Display for TokenPaymentByBurningOnlyAllowedOnInternalTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Token payments by burning only allowed on internal tokens. \
Trying to register a contract that burns tokens with id {} from contract {} \
at position {} for action '{}'.",
            self.external_token_id(),
            self.external_token_contract_id,
            self.external_token_contract_token_position,
            self.action
        )
    }
}

impl From<TokenPaymentByBurningOnlyAllowedOnInternalTokenError> for ConsensusError {
    fn from(err: TokenPaymentByBurningOnlyAllowedOnInternalTokenError) -> Self {
        Self::BasicError(BasicError::TokenPaymentByBurningOnlyAllowedOnInternalTokenError(err))
    }
}
