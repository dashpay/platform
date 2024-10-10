use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("There is no transfer key that can be used for a withdrawal")]
#[platform_serialize(unversioned)]
pub struct NoTransferKeyForCoreWithdrawalAvailableError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    identity_id: Identifier,
}

impl NoTransferKeyForCoreWithdrawalAvailableError {
    pub fn new(identity_id: Identifier) -> Self {
        Self { identity_id }
    }

    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}
impl From<NoTransferKeyForCoreWithdrawalAvailableError> for ConsensusError {
    fn from(err: NoTransferKeyForCoreWithdrawalAvailableError) -> Self {
        Self::StateError(StateError::NoTransferKeyForCoreWithdrawalAvailableError(
            err,
        ))
    }
}
