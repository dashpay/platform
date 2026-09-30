use crate::consensus::state::identity::identity_public_key_limit_not_set_error::KeyLimit;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
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
#[error("Identity public key {public_key_id} {limit} is not raised: the requested value {requested} is not greater than the current {current}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyLimitNotRaisedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    limit: KeyLimit,
    current: u64,
    requested: u64,
}

impl IdentityPublicKeyLimitNotRaisedError {
    pub fn new(public_key_id: KeyID, limit: KeyLimit, current: u64, requested: u64) -> Self {
        Self {
            public_key_id,
            limit,
            current,
            requested,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn limit(&self) -> KeyLimit {
        self.limit
    }

    /// The value the key holds now
    pub fn current(&self) -> u64 {
        self.current
    }

    /// The value the transition asked for
    pub fn requested(&self) -> u64 {
        self.requested
    }
}

impl From<IdentityPublicKeyLimitNotRaisedError> for ConsensusError {
    fn from(err: IdentityPublicKeyLimitNotRaisedError) -> Self {
        Self::StateError(StateError::IdentityPublicKeyLimitNotRaisedError(err))
    }
}
