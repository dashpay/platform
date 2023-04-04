use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::SecurityLevel;

use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Invalid security level {public_key_security_level}. This state transition requires at least {required_security_level}")]
pub struct PublicKeySecurityLevelNotMetError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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
        self.public_key_security_level
    }
    pub fn required_security_level(&self) -> SecurityLevel {
        self.required_security_level
    }
}

impl From<PublicKeySecurityLevelNotMetError> for ConsensusError {
    fn from(err: PublicKeySecurityLevelNotMetError) -> Self {
        Self::SignatureError(SignatureError::PublicKeySecurityLevelNotMetError(err))
    }
}
