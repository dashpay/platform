use crate::consensus::basic::BasicError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A non-transferable token cannot be tradeable: filling a listing or an offer moves the token
/// from the seller to the buyer, which is the transfer the token refuses.
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
    "Token at position {token_contract_position} is not transferable, so it cannot be tradeable: filling a listing or an offer moves the token to another identity"
)]
#[platform_serialize(unversioned)]
pub struct NonTransferableTokenTradeableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_contract_position: TokenContractPosition,
}

impl NonTransferableTokenTradeableError {
    pub fn new(token_contract_position: TokenContractPosition) -> Self {
        Self {
            token_contract_position,
        }
    }

    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }
}

impl From<NonTransferableTokenTradeableError> for ConsensusError {
    fn from(err: NonTransferableTokenTradeableError) -> Self {
        Self::BasicError(BasicError::NonTransferableTokenTradeableError(err))
    }
}
