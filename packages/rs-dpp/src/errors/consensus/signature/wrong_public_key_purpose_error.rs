use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::Purpose;

use serde::{Deserialize, Serialize};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        self.public_key_purpose
    }
    pub fn key_purpose_requirement(&self) -> Purpose {
        self.key_purpose_requirement
    }
}

impl From<WrongPublicKeyPurposeError> for ConsensusError {
    fn from(err: WrongPublicKeyPurposeError) -> Self {
        Self::SignatureError(SignatureError::WrongPublicKeyPurposeError(err))
    }
}
