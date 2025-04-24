use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::Identifier;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Token claim error: Identity '{}' is not a valid claimant for this distribution type of token '{}'. The valid claimaint is '{}'.",
    claimant_id,
    token_id,
    expected_claimant_id
)]
#[platform_serialize(unversioned)]
pub struct InvalidTokenClaimWrongClaimant {
    token_id: Identifier,
    expected_claimant_id: Identifier,
    claimant_id: Identifier,
}

impl InvalidTokenClaimWrongClaimant {
    /// Creates a new `InvalidTokenClaimWrongClaimant` error.
    pub fn new(
        token_id: Identifier,
        expected_claimant_id: Identifier,
        claimant_id: Identifier,
    ) -> Self {
        Self {
            token_id,
            expected_claimant_id,
            claimant_id,
        }
    }

    /// Returns the token ID associated with the error.
    pub fn token_id(&self) -> Identifier {
        self.token_id
    }

    /// Returns the expected claimant ID.
    pub fn expected_claimant_id(&self) -> Identifier {
        self.expected_claimant_id
    }

    /// Returns the actual claimant ID.
    pub fn claimant_id(&self) -> Identifier {
        self.claimant_id
    }
}

/// Implement conversion from `InvalidTokenClaimWrongClaimant` to `ConsensusError`.
impl From<InvalidTokenClaimWrongClaimant> for ConsensusError {
    fn from(err: InvalidTokenClaimWrongClaimant) -> Self {
        Self::StateError(StateError::InvalidTokenClaimWrongClaimant(err))
    }
}
