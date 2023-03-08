use thiserror::Error;

use crate::identity::SecurityLevel;
use crate::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid key security level: {public_key_security_level}. The state transition requires at least: {required_security_level}")]
pub struct PublicKeySecurityLevelNotMetError {
    public_key_security_level: SecurityLevel,
    required_security_level: SecurityLevel,
}

impl PublicKeySecurityLevelNotMetError {
    pub fn new(
        public_key_security_level: SecurityLevel,
        required_security_level: SecurityLevel,
    ) -> Self {
        Self {
            public_key_security_level,
            required_security_level,
        }
    }

    pub fn public_key_security_level(&self) -> SecurityLevel {
        self.public_key_security_level.clone()
    }
    pub fn required_security_level(&self) -> SecurityLevel {
        self.required_security_level.clone()
    }
}

impl From<PublicKeySecurityLevelNotMetError> for ProtocolError {
    fn from(err: PublicKeySecurityLevelNotMetError) -> Self {
        Self::PublicKeySecurityLevelNotMetError(err)
    }
}
