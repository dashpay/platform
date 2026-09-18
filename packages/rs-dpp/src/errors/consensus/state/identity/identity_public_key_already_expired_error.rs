use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::{KeyID, TimestampMillis};
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
#[error("Identity public key {public_key_id} is expired at the block time: it expires at {expires_at} ms and the block time is {block_time_ms} ms")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyAlreadyExpiredError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    expires_at: TimestampMillis,
    block_time_ms: TimestampMillis,
}

impl IdentityPublicKeyAlreadyExpiredError {
    pub fn new(
        public_key_id: KeyID,
        expires_at: TimestampMillis,
        block_time_ms: TimestampMillis,
    ) -> Self {
        Self {
            public_key_id,
            expires_at,
            block_time_ms,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn expires_at(&self) -> TimestampMillis {
        self.expires_at
    }

    pub fn block_time_ms(&self) -> TimestampMillis {
        self.block_time_ms
    }
}

impl From<IdentityPublicKeyAlreadyExpiredError> for ConsensusError {
    fn from(err: IdentityPublicKeyAlreadyExpiredError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyAlreadyExpiredError(err))
    }
}
