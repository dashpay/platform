use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use std::fmt;
use thiserror::Error;

/// One of the two limits an authentication key may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, DecodeUntrusted)]
pub enum KeyLimit {
    /// The total budget of the key
    Budget,
    /// The expiry of the key
    Expiry,
}

impl fmt::Display for KeyLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyLimit::Budget => write!(f, "budget"),
            KeyLimit::Expiry => write!(f, "expiry"),
        }
    }
}

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
#[error("Identity public key {public_key_id} has no {limit} to raise: a limit can only be raised, never added")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyLimitNotSetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    limit: KeyLimit,
}

impl IdentityPublicKeyLimitNotSetError {
    pub fn new(public_key_id: KeyID, limit: KeyLimit) -> Self {
        Self {
            public_key_id,
            limit,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn limit(&self) -> KeyLimit {
        self.limit
    }
}

impl From<IdentityPublicKeyLimitNotSetError> for ConsensusError {
    fn from(err: IdentityPublicKeyLimitNotSetError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyLimitNotSetError(err))
    }
}
