use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use crate::identity::Purpose;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Key purpose {given_key_purpose} is not allowed for contract bounds. Allowed purposes: {allowed_key_purposes:?}")]
#[platform_serialize(unversioned)]
pub struct InvalidKeyPurposeForContractBoundsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    given_key_purpose: Purpose,
    allowed_key_purposes: Vec<Purpose>,
}

impl InvalidKeyPurposeForContractBoundsError {
    pub fn new(given_key_purpose: Purpose, allowed_key_purposes: Vec<Purpose>) -> Self {
        Self {
            given_key_purpose,
            allowed_key_purposes,
        }
    }

    pub fn given_key_purpose(&self) -> Purpose {
        self.given_key_purpose
    }

    pub fn allowed_key_purposes(&self) -> &Vec<Purpose> {
        &self.allowed_key_purposes
    }
}

impl From<InvalidKeyPurposeForContractBoundsError> for ConsensusError {
    fn from(err: InvalidKeyPurposeForContractBoundsError) -> Self {
        Self::BasicError(BasicError::InvalidKeyPurposeForContractBoundsError(err))
    }
}
