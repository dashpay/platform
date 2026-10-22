use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A token order's expiry is a block time in milliseconds that the order's document stores as a
/// signed 8-byte value, so it is at most `i64::MAX`. Whether it is after the block is checked
/// when the order is placed.
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
#[error("Token order expiry {expires_at} must be at most {max_expires_at} milliseconds")]
#[platform_serialize(unversioned)]
pub struct InvalidTokenOrderExpiryError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    expires_at: TimestampMillis,
    max_expires_at: TimestampMillis,
}

impl InvalidTokenOrderExpiryError {
    pub fn new(expires_at: TimestampMillis, max_expires_at: TimestampMillis) -> Self {
        Self {
            expires_at,
            max_expires_at,
        }
    }

    pub fn expires_at(&self) -> TimestampMillis {
        self.expires_at
    }

    pub fn max_expires_at(&self) -> TimestampMillis {
        self.max_expires_at
    }
}

impl From<InvalidTokenOrderExpiryError> for ConsensusError {
    fn from(err: InvalidTokenOrderExpiryError) -> Self {
        Self::BasicError(BasicError::InvalidTokenOrderExpiryError(err))
    }
}
