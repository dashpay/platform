use thiserror::Error;

use crate::identity::Purpose;
use crate::ProtocolError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid identity key purpose {public_key_purpose}. This state transition requires {key_purpose_requirement}")]
pub struct WrongPublicKeyPurposeError {
    public_key_purpose: Purpose,
    key_purpose_requirement: Purpose,
}

impl WrongPublicKeyPurposeError {
    pub fn new(public_key_purpose: Purpose, key_purpose_requirement: Purpose) -> Self {
        Self {
            public_key_purpose,
            key_purpose_requirement,
        }
    }

    pub fn public_key_purpose(&self) -> Purpose {
        self.public_key_purpose.clone()
    }
    pub fn key_purpose_requirement(&self) -> Purpose {
        self.key_purpose_requirement.clone()
    }
}

impl From<WrongPublicKeyPurposeError> for ProtocolError {
    fn from(err: WrongPublicKeyPurposeError) -> Self {
        Self::WrongPublicKeyPurposeError(err)
    }
}
