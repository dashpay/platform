use thiserror::Error;
use crate::errors::consensus::AbstractConsensusError;
use crate::identity::{Purpose, SecurityLevel};
use crate::PublicKeyValidationError;

#[derive(Error, Debug, Clone)]
#[error("Invalid identity public key {public_key_id:?} security level: purpose {purpose:?} allows only for {allowed_security_levels:?} security levels, but got {security_level:?}")]
pub struct InvalidIdentityPublicKeySecurityLevelError {
    public_key_id: u64,
    purpose: Purpose,
    security_level: SecurityLevel,
    allowed_security_levels: String
}

impl InvalidIdentityPublicKeySecurityLevelError {
    pub fn new(public_key_id: u64, purpose: Purpose, security_level: SecurityLevel, allowed_security_levels: Option<Vec<SecurityLevel>>) -> Self {
        Self {
            public_key_id,
            purpose,
            security_level,
            allowed_security_levels: allowed_security_levels.map_or("".into_string(), |levels| {
                format!("{:?}", levels)
                // levels.iter().fold(String::new(), |mut sum, val| { sum.push_str(format!("{}", val.to_s)) })
            })
        }
    }

    pub fn public_key_id(&self) -> u64 {
        self.public_key_id
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn security_level(&self) -> SecurityLevel {
        self.security_level
    }
}
