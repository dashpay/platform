use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid shielded value_balance: {message}")]
#[platform_serialize(unversioned)]
pub struct ShieldedInvalidValueBalanceError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    /// A human-readable description of the validation failure (e.g. "must be negative",
    /// "must be positive", "must be zero"). Ideally this would be typed fields such as
    /// `value_balance: i64` and `reason: &'static str`, but because this struct derives
    /// `PlatformSerialize`/`Encode` and is part of the consensus protocol (error code 10822),
    /// changing the field layout would break wire compatibility. Use a new version instead.
    message: String,
}

impl ShieldedInvalidValueBalanceError {
    pub fn new(message: String) -> Self {
        Self { message }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<ShieldedInvalidValueBalanceError> for ConsensusError {
    fn from(err: ShieldedInvalidValueBalanceError) -> Self {
        Self::BasicError(BasicError::ShieldedInvalidValueBalanceError(err))
    }
}
