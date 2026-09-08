use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("invalid propertyAgreement pair {referring_property} -> {referenced_property} declared at {path}: {reason}")]
#[platform_serialize(unversioned)]
pub struct ReferencedDocumentPropertyAgreementInvalidError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    path: String,
    referring_property: String,
    referenced_property: String,
    reason: String,
}

impl ReferencedDocumentPropertyAgreementInvalidError {
    pub fn new(
        path: String,
        referring_property: String,
        referenced_property: String,
        reason: String,
    ) -> Self {
        Self {
            path,
            referring_property,
            referenced_property,
            reason,
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

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

impl From<ReferencedDocumentPropertyAgreementInvalidError> for ConsensusError {
    fn from(err: ReferencedDocumentPropertyAgreementInvalidError) -> Self {
        Self::StateError(StateError::ReferencedDocumentPropertyAgreementInvalidError(
            err,
        ))
    }
}
