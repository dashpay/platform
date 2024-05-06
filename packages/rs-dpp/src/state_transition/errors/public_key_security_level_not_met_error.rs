use thiserror::Error;

use crate::identity::identity_public_key::SecurityLevel;
use crate::errors::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid key security level: {public_key_security_level}. The state transition requires at least: {required_security_level}")]
#[ferment_macro::export]
pub struct PublicKeySecurityLevelNotMetError {
    pub public_key_security_level: SecurityLevel,
    pub required_security_level: SecurityLevel,
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
        self.public_key_security_level
    }
    pub fn required_security_level(&self) -> SecurityLevel {
        self.required_security_level
    }
}

impl From<PublicKeySecurityLevelNotMetError> for ProtocolError {
    fn from(err: PublicKeySecurityLevelNotMetError) -> Self {
        Self::PublicKeySecurityLevelNotMetError(err)
    }
}
