use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::prelude::Revision;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Document {document_id} has invalid revision {previous_revision:?}. The desired revision is {desired_revision}"
)]
#[platform_serialize(unversioned)]
pub struct InvalidDocumentRevisionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    document_id: Identifier,
    previous_revision: Option<Revision>,
    desired_revision: Revision,
}

impl InvalidDocumentRevisionError {
    pub fn new(
        document_id: Identifier,
        previous_revision: Option<Revision>,
        desired_revision: Revision,
    ) -> Self {
        Self {
            document_id,
            previous_revision,
            desired_revision,
        }
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn previous_revision(&self) -> &Option<Revision> {
        &self.previous_revision
    }

    pub fn desired_revision(&self) -> &Revision {
        &self.desired_revision
    }
}

impl From<InvalidDocumentRevisionError> for ConsensusError {
    fn from(err: InvalidDocumentRevisionError) -> Self {
        Self::StateError(StateError::InvalidDocumentRevisionError(err))
    }
}
