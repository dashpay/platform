use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Token claim error: expected claimant '{}' for token ID '{}', but received claim from '{}'",
    expected_claimant_id,
    token_id,
    claimant_id
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct InvalidTokenClaimWrongClaimant {
    pub token_id: Identifier,
    pub expected_claimant_id: Identifier,
    pub claimant_id: Identifier,
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
