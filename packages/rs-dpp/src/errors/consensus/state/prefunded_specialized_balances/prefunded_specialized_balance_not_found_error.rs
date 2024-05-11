use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::prelude::Identifier;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Did not find a specialized balance with id: {balance_id}")]
#[platform_serialize(unversioned)]
pub struct PrefundedSpecializedBalanceNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub balance_id: Identifier,
}

impl PrefundedSpecializedBalanceNotFoundError {
    pub fn new(balance_id: Identifier) -> Self {
        Self { balance_id }
    }

    pub fn balance_id(&self) -> &Identifier {
        &self.balance_id
    }
}
impl From<PrefundedSpecializedBalanceNotFoundError> for ConsensusError {
    fn from(err: PrefundedSpecializedBalanceNotFoundError) -> Self {
        Self::StateError(StateError::PrefundedSpecializedBalanceNotFoundError(err))
    }
}
