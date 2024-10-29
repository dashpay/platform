use thiserror::Error;

use crate::consensus::signature::signature_error::SignatureError;
use crate::consensus::ConsensusError;
use crate::identity::Purpose;

use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};

use bincode::{Decode, Encode};
use itertools::Itertools;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Invalid identity key purpose {public_key_purpose}. This state transition requires {}", allowed_key_purposes.iter().map(|s| s.to_string()).join(" | "))]
#[platform_serialize(unversioned)]
pub struct WrongPublicKeyPurposeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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

impl From<WrongPublicKeyPurposeError> for ConsensusError {
    fn from(err: WrongPublicKeyPurposeError) -> Self {
        Self::SignatureError(SignatureError::WrongPublicKeyPurposeError(err))
    }
}

#[cfg(any(
    all(feature = "state-transitions", feature = "validation"),
    feature = "state-transition-validation"
))]
// This is a separate error of the same name, but from ProtocolError
impl From<crate::state_transition::errors::WrongPublicKeyPurposeError> for ConsensusError {
    fn from(value: crate::state_transition::errors::WrongPublicKeyPurposeError) -> Self {
        Self::SignatureError(SignatureError::WrongPublicKeyPurposeError(
            WrongPublicKeyPurposeError::new(
                value.public_key_purpose(),
                value.allowed_key_purposes().clone(),
            ),
        ))
    }
}
