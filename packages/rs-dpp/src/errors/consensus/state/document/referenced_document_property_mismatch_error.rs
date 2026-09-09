use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("the document's {referring_property} does not agree with the referenced document's {referenced_property} (propertyAgreement on {path})")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentPropertyMismatchError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    path: String,
    referring_property: String,
    referenced_property: String,
}

impl ReferencedDocumentPropertyMismatchError {
    pub fn new(path: String, referring_property: String, referenced_property: String) -> Self {
        Self {
            path,
            referring_property,
            referenced_property,
        }
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn referring_property(&self) -> &str {
        &self.referring_property
    }

    pub fn referenced_property(&self) -> &str {
        &self.referenced_property
    }
}

impl From<ReferencedDocumentPropertyMismatchError> for ConsensusError {
    fn from(err: ReferencedDocumentPropertyMismatchError) -> Self {
        Self::StateError(StateError::ReferencedDocumentPropertyMismatchError(err))
    }
}
