use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::{KeyID, KeyType, Purpose};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A key registered for a purpose its key type cannot serve: an ML_DSA_65 key signs, so it is
/// refused for ENCRYPTION and DECRYPTION.
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
#[error("Identity public key {public_key_id} has the key type {key_type}, which cannot be used for the purpose {purpose}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyTypeNotAllowedForPurposeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    key_type: KeyType,
    purpose: Purpose,
}

impl IdentityPublicKeyTypeNotAllowedForPurposeError {
    pub fn new(public_key_id: KeyID, key_type: KeyType, purpose: Purpose) -> Self {
        Self {
            public_key_id,
            key_type,
            purpose,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }
}

impl From<IdentityPublicKeyTypeNotAllowedForPurposeError> for ConsensusError {
    fn from(err: IdentityPublicKeyTypeNotAllowedForPurposeError) -> Self {
        Self::BasicError(BasicError::IdentityPublicKeyTypeNotAllowedForPurposeError(
            err,
        ))
    }
}
