use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A token mint asking for a random amount (protocol version 15) that cannot take one: it is a
/// group action, whose members would weigh an amount nobody knows, or it leaves its recipient
/// to the token's configuration, which the payout must not read after the block executes the
/// mint. `reason` says which.
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
#[error("A mint of token {token_id} cannot issue a random amount: {reason}")]
#[platform_serialize(unversioned)]
pub struct RandomTokenMintNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    token_id: Identifier,
    reason: String,
}

impl RandomTokenMintNotAllowedError {
    pub fn new(token_id: Identifier, reason: String) -> Self {
        Self { token_id, reason }
    }

    pub fn token_id(&self) -> Identifier {
        self.token_id
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<RandomTokenMintNotAllowedError> for ConsensusError {
    fn from(err: RandomTokenMintNotAllowedError) -> Self {
        Self::BasicError(BasicError::RandomTokenMintNotAllowedError(err))
    }
}
