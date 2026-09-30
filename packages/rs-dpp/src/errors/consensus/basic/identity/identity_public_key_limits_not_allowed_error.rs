use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::{KeyID, Purpose, SecurityLevel};
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
#[error("Identity public key {public_key_id} cannot carry a budget or an expiry: limits are only allowed on AUTHENTICATION keys below the MASTER security level, but got purpose {purpose:?} and security level {security_level:?}")]
#[platform_serialize(unversioned)]
pub struct IdentityPublicKeyLimitsNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    purpose: Purpose,
    security_level: SecurityLevel,
}

impl IdentityPublicKeyLimitsNotAllowedError {
    pub fn new(public_key_id: KeyID, purpose: Purpose, security_level: SecurityLevel) -> Self {
        Self {
            public_key_id,
            purpose,
            security_level,
        }
    }

    pub fn public_key_id(&self) -> KeyID {
        self.public_key_id
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }
}

impl From<IdentityPublicKeyLimitsNotAllowedError> for ConsensusError {
    fn from(err: IdentityPublicKeyLimitsNotAllowedError) -> Self {
        Self::BasicError(BasicError::IdentityPublicKeyLimitsNotAllowedError(err))
    }
}
