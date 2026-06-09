use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid shielded identity-create denomination {denomination}: must be one of the allowed exit denominations")]
#[platform_serialize(unversioned)]
pub struct ShieldedInvalidDenominationError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    /// The rejected denomination (in credits). `IdentityCreateFromShieldedPool` may only exit one of
    /// a small versioned set of fixed denominations so every exit of a given size is indistinguishable
    /// on-chain (maximizing the anonymity set). Any other value — including a non-member amount or a
    /// `value_balance` that does not equal the declared denomination — is rejected with this error
    /// (consensus error code 10827).
    denomination: u64,
}

impl ShieldedInvalidDenominationError {
    pub fn new(denomination: u64) -> Self {
        Self { denomination }
    }

    pub fn denomination(&self) -> u64 {
        self.denomination
    }
}

impl From<ShieldedInvalidDenominationError> for ConsensusError {
    fn from(err: ShieldedInvalidDenominationError) -> Self {
        Self::BasicError(BasicError::ShieldedInvalidDenominationError(err))
    }
}
