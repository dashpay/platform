use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::KeyID;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A transition adds more than one HECATE_SIGNER key: an identity holds at most one enabled
/// (protocol version 15).
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
#[error("Identity public keys {public_key_ids:?} are all HECATE_SIGNER keys: an identity holds at most one enabled")]
#[platform_serialize(unversioned)]
pub struct TooManyHecateSignerKeysError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_ids: Vec<KeyID>,
}

impl TooManyHecateSignerKeysError {
    pub fn new(public_key_ids: Vec<KeyID>) -> Self {
        Self { public_key_ids }
    }

    pub fn public_key_ids(&self) -> &[KeyID] {
        &self.public_key_ids
    }
}

impl From<TooManyHecateSignerKeysError> for ConsensusError {
    fn from(err: TooManyHecateSignerKeysError) -> Self {
        Self::BasicError(BasicError::TooManyHecateSignerKeysError(err))
    }
}
