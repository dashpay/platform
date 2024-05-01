use itertools::Itertools;
use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::SecurityLevel;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid public key security level {public_key_security_level}. The state transition requires one of {}", allowed_key_security_levels.iter().map(|s| s.to_string()).join(" | "))]
#[platform_serialize(unversioned)]
pub struct InvalidSignaturePublicKeySecurityLevelError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_security_level: SecurityLevel,
    allowed_key_security_levels: Vec<SecurityLevel>,
}

impl InvalidSignaturePublicKeySecurityLevelError {
    pub fn new(
        public_key_security_level: SecurityLevel,
        allowed_key_security_levels: Vec<SecurityLevel>,
    ) -> Self {
        Self {
            public_key_security_level,
            allowed_key_security_levels,
        }
    }

    pub fn public_key_security_level(&self) -> SecurityLevel {
        self.public_key_security_level
    }
    pub fn allowed_key_security_levels(&self) -> Vec<SecurityLevel> {
        self.allowed_key_security_levels.clone()
    }
}

impl From<InvalidSignaturePublicKeySecurityLevelError> for ConsensusError {
    fn from(err: InvalidSignaturePublicKeySecurityLevelError) -> Self {
        Self::SignatureError(SignatureError::InvalidSignaturePublicKeySecurityLevelError(
            err,
        ))
    }
}
