use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Shielded transition has {actual_actions} actions, which exceeds the maximum allowed {max_actions}"
)]
#[platform_serialize(unversioned)]
pub struct ShieldedTooManyActionsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    actual_actions: u16,
    max_actions: u16,
}

impl ShieldedTooManyActionsError {
    pub fn new(actual_actions: u16, max_actions: u16) -> Self {
        Self {
            actual_actions,
            max_actions,
        }
    }

    pub fn actual_actions(&self) -> u16 {
        self.actual_actions
    }

    pub fn max_actions(&self) -> u16 {
        self.max_actions
    }
}

impl From<ShieldedTooManyActionsError> for ConsensusError {
    fn from(err: ShieldedTooManyActionsError) -> Self {
        Self::BasicError(BasicError::ShieldedTooManyActionsError(err))
    }
}
