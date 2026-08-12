use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use crate::identity::{KeyID, KeyType, Purpose};
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Identity key {key_id} of purpose {purpose} can not use key type {key_type}. Allowed key types: {allowed_key_types:?}")]
#[platform_serialize(unversioned)]
pub struct InvalidKeyPurposeKeyTypeError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_id: KeyID,
    purpose: Purpose,
    key_type: KeyType,
    allowed_key_types: Vec<KeyType>,
}

impl InvalidKeyPurposeKeyTypeError {
    pub fn new(
        key_id: KeyID,
        purpose: Purpose,
        key_type: KeyType,
        allowed_key_types: Vec<KeyType>,
    ) -> Self {
        Self {
            key_id,
            purpose,
            key_type,
            allowed_key_types,
        }
    }

    pub fn key_id(&self) -> KeyID {
        self.key_id
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn key_type(&self) -> KeyType {
        self.key_type
    }

    pub fn allowed_key_types(&self) -> &Vec<KeyType> {
        &self.allowed_key_types
    }
}

impl From<InvalidKeyPurposeKeyTypeError> for ConsensusError {
    fn from(err: InvalidKeyPurposeKeyTypeError) -> Self {
        Self::BasicError(BasicError::InvalidKeyPurposeKeyTypeError(err))
    }
}
