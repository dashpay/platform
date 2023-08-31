use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::identity::{KeyID, Purpose, SecurityLevel};

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid identity public key {public_key_id:?} security level: purpose {purpose:?} allows only for {allowed_security_levels:?} security levels, but got {security_level:?}")]
#[platform_serialize(unversioned)]
pub struct InvalidIdentityPublicKeySecurityLevelError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    public_key_id: KeyID,
    purpose: Purpose,
    security_level: SecurityLevel,
    allowed_security_levels: String,
}

impl InvalidIdentityPublicKeySecurityLevelError {
    pub fn new(
        public_key_id: KeyID,
        purpose: Purpose,
        security_level: SecurityLevel,
        allowed_security_levels: Option<Vec<SecurityLevel>>,
    ) -> Self {
        Self {
            public_key_id,
            purpose,
            security_level,
            allowed_security_levels: allowed_security_levels
                .map_or(String::from(""), |levels| format!("{:?}", levels)),
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

impl From<InvalidIdentityPublicKeySecurityLevelError> for ConsensusError {
    fn from(err: InvalidIdentityPublicKeySecurityLevelError) -> Self {
        Self::BasicError(BasicError::InvalidIdentityPublicKeySecurityLevelError(err))
    }
}
