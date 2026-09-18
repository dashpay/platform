use crate::consensus::basic::BasicError;
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
#[error("Identity key limits update of key {public_key_id} changes nothing: it must set a new total budget, a new expiry, or both")]
#[platform_serialize(unversioned)]
pub struct IdentityKeyLimitsUpdateEmptyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
}

impl IdentityKeyLimitsUpdateEmptyError {
    pub fn new(public_key_id: KeyID) -> Self {
        Self { public_key_id }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }
}

impl From<IdentityKeyLimitsUpdateEmptyError> for ConsensusError {
    fn from(err: IdentityKeyLimitsUpdateEmptyError) -> Self {
        Self::BasicError(BasicError::IdentityKeyLimitsUpdateEmptyError(err))
    }
}
