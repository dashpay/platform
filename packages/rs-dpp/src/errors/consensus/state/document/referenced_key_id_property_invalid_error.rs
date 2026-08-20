use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("keyIdProperty {key_id_property} referenced at path {path} is invalid: {message}")]
#[platform_serialize(unversioned)]
pub struct ReferencedKeyIdPropertyInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    key_id_property: String,
    path: String,
    message: String,
}

impl ReferencedKeyIdPropertyInvalidError {
    pub fn new(key_id_property: String, path: String, message: String) -> Self {
        Self {
            key_id_property,
            path,
            message,
        }
    }

    pub fn key_id_property(&self) -> &str {
        &self.key_id_property
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<ReferencedKeyIdPropertyInvalidError> for ConsensusError {
    fn from(err: ReferencedKeyIdPropertyInvalidError) -> Self {
        Self::StateError(StateError::ReferencedKeyIdPropertyInvalidError(err))
    }
}
