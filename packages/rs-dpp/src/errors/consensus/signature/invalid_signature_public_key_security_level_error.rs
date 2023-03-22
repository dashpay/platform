use thiserror::Error;

use crate::consensus::signature::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::SecurityLevel;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid public key security level {public_key_security_level}. The state transition requires {required_key_security_level}")]
pub struct InvalidSignaturePublicKeySecurityLevelError {
    public_key_security_level: SecurityLevel,
    required_key_security_level: SecurityLevel,
}

impl InvalidSignaturePublicKeySecurityLevelError {
    pub fn new(
        public_key_security_level: SecurityLevel,
        required_key_security_level: SecurityLevel,
    ) -> Self {
        Self {
            public_key_security_level,
            required_key_security_level,
        }
    }

    pub fn public_key_security_level(&self) -> SecurityLevel {
        self.public_key_security_level
    }
    pub fn required_key_security_level(&self) -> SecurityLevel {
        self.required_key_security_level
    }
}

impl From<InvalidSignaturePublicKeySecurityLevelError> for ConsensusError {
    fn from(err: InvalidSignaturePublicKeySecurityLevelError) -> Self {
        Self::SignatureError(SignatureError::InvalidSignaturePublicKeySecurityLevelError(
            err,
        ))
    }
}
