use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::TimestampMillis;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A token order's expiry has passed, or a new order expires before it could be filled.
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
#[error("Token order {order_id} expires at {expires_at}, at or before the block time {block_time}")]
#[platform_serialize(unversioned)]
pub struct TokenOrderExpiredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    order_id: Identifier,
    expires_at: TimestampMillis,
    block_time: TimestampMillis,
}

impl TokenOrderExpiredError {
    pub fn new(
        order_id: Identifier,
        expires_at: TimestampMillis,
        block_time: TimestampMillis,
    ) -> Self {
        Self {
            order_id,
            expires_at,
            block_time,
        }
    }

    pub fn order_id(&self) -> Identifier {
        self.order_id
    }

    pub fn expires_at(&self) -> TimestampMillis {
        self.expires_at
    }

    pub fn block_time(&self) -> TimestampMillis {
        self.block_time
    }
}

impl From<TokenOrderExpiredError> for ConsensusError {
    fn from(err: TokenOrderExpiredError) -> Self {
        Self::StateError(StateError::TokenOrderExpiredError(err))
    }
}
