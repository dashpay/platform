use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::{KeyID, KeyType, Purpose};
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use thiserror::Error;

/// A key whose type and purpose do not go together: a REDPALLAS_HECATE key must have the
/// HECATE_SIGNER purpose, and a HECATE_SIGNER key must be a REDPALLAS_HECATE key (protocol
/// version 15).
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
#[error("Identity public key {public_key_id} has key type {key_type} and purpose {purpose}: a REDPALLAS_HECATE key must have the HECATE_SIGNER purpose, and a HECATE_SIGNER key must be a REDPALLAS_HECATE key")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeyTypeForPurposeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    key_type: KeyType,
    purpose: Purpose,
}

impl InvalidIdentityPublicKeyTypeForPurposeError {
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

impl From<InvalidIdentityPublicKeyTypeForPurposeError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeyTypeForPurposeError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityPublicKeyTypeForPurposeError(err))
    }
}
