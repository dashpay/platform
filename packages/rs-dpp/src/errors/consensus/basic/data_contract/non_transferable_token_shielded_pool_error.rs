use crate::consensus::basic::BasicError;
use crate::data_contract::TokenContractPosition;
use crate::errors::ProtocolError;
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, DecodeUntrusted, Encode};

/// A non-transferable token cannot have a shielded pool: a note spent in the pool can pay out to
/// any identity, so shielding and unshielding would move the token between holders.
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
    "Token at position {token_contract_position} is not transferable, so it cannot have a shielded pool: shielded notes can be unshielded to any identity"
)]
#[platform_serialize(unversioned)]
pub struct NonTransferableTokenShieldedPoolError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_contract_position: TokenContractPosition,
}

impl NonTransferableTokenShieldedPoolError {
    pub fn new(token_contract_position: TokenContractPosition) -> Self {
        Self {
            token_contract_position,
        }
    }

    pub fn token_contract_position(&self) -> TokenContractPosition {
        self.token_contract_position
    }
}

impl From<NonTransferableTokenShieldedPoolError> for ConsensusError {
    fn from(err: NonTransferableTokenShieldedPoolError) -> Self {
        Self::BasicError(BasicError::NonTransferableTokenShieldedPoolError(err))
    }
}
