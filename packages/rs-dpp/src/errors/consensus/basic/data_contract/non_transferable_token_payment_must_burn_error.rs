use crate::consensus::basic::BasicError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A document type charges its contract's own non-transferable token without burning it. Paying
/// the contract owner would move the token to another identity, so the cost must use the burn
/// effect.
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
    "Token at position {token_contract_position} is not transferable, so the {action} token cost must burn it instead of paying the contract owner"
)]
#[platform_serialize(unversioned)]
pub struct NonTransferableTokenPaymentMustBurnError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_contract_position: TokenContractPosition,
    action: String,
}

impl NonTransferableTokenPaymentMustBurnError {
    pub fn new(token_contract_position: TokenContractPosition, action: String) -> Self {
        Self {
            token_contract_position,
            action,
        }
    }

    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<NonTransferableTokenPaymentMustBurnError> for ConsensusError {
    fn from(err: NonTransferableTokenPaymentMustBurnError) -> Self {
        Self::BasicError(BasicError::NonTransferableTokenPaymentMustBurnError(err))
    }
}
