use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity public keys disabled time ({disabled_at}) is out of block time window from {time_window_start} and {time_window_end}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyDisabledAtWindowViolationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    disabled_at: u64,
    time_window_start: u64,
    time_window_end: u64,
}

impl IdentityPublicKeyDisabledAtWindowViolationError {
    pub fn new(disabled_at: u64, time_window_start: u64, time_window_end: u64) -> Self {
        Self {
            disabled_at,
            time_window_start,
            time_window_end,
        }
    }

    pub fn disabled_at(&self) -> u64 {
        self.disabled_at
    }

    pub fn time_window_start(&self) -> u64 {
        self.time_window_start
    }
    pub fn time_window_end(&self) -> u64 {
        self.time_window_end
    }
}
impl From<IdentityPublicKeyDisabledAtWindowViolationError> for ConsensusError {
    fn from(err: IdentityPublicKeyDisabledAtWindowViolationError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyDisabledAtWindowViolationError(
            err,
        ))
    }
}
