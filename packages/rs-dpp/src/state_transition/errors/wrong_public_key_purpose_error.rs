use thiserror::Error;

use crate::identity::Purpose;
use crate::ProtocolError;
use itertools::Itertools;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Invalid identity key purpose {public_key_purpose}. This state transition requires {}", allowed_key_purposes.iter().map(|s| s.to_string()).join(" | "))]
pub struct WrongPublicKeyPurposeError {
    public_key_purpose: Purpose,
    allowed_key_purposes: Vec<Purpose>,
}

impl WrongPublicKeyPurposeError {
    pub fn new(public_key_purpose: Purpose, allowed_key_purposes: Vec<Purpose>) -> Self {
        Self {
            public_key_purpose,
            allowed_key_purposes,
        }
    }

    pub fn public_key_purpose(&self) -> Purpose {
        self.public_key_purpose
    }
    pub fn allowed_key_purposes(&self) -> &Vec<Purpose> {
        &self.allowed_key_purposes
    }
}

impl From<WrongPublicKeyPurposeError> for ProtocolError {
    fn from(err: WrongPublicKeyPurposeError) -> Self {
        Self::WrongPublicKeyPurposeError(err)
    }
}
