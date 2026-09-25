use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::{Identifier, TimestampMillis};
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

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
    "Token claim error: identity '{identity_id}' already claimed the once-per-identity distribution of token '{token_id}' at {claimed_at_ms}"
)]
#[platform_serialize(unversioned)]
pub struct TokenOncePerIdentityDistributionAlreadyClaimedError {
    token_id: Identifier,
    identity_id: Identifier,
    claimed_at_ms: TimestampMillis,
}

impl TokenOncePerIdentityDistributionAlreadyClaimedError {
    pub fn new(
        token_id: Identifier,
        identity_id: Identifier,
        claimed_at_ms: TimestampMillis,
    ) -> Self {
        Self {
            token_id,
            identity_id,
            claimed_at_ms,
        }
    }

    pub fn token_id(&self) -> Identifier {
        self.token_id
    }

    pub fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    /// The block time, in milliseconds, of the claim that was already paid.
    pub fn claimed_at_ms(&self) -> TimestampMillis {
        self.claimed_at_ms
    }
}

impl From<TokenOncePerIdentityDistributionAlreadyClaimedError> for ConsensusError {
    fn from(err: TokenOncePerIdentityDistributionAlreadyClaimedError) -> Self {
        Self::StateError(StateError::TokenOncePerIdentityDistributionAlreadyClaimedError(err))
    }
}
