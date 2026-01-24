use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::data_contract::document_type::property::reference::DocumentPropertyReferenceTarget;
use crate::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("referenced {entity_type} {entity_id} not found for path {path}")]
#[platform_serialize(unversioned)]
pub struct ReferencedEntityNotFoundError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    entity_id: Identifier,
    entity_type: DocumentPropertyReferenceTarget,
    path: String,
}

impl ReferencedEntityNotFoundError {
    pub fn new(
        entity_id: Identifier,
        entity_type: DocumentPropertyReferenceTarget,
        path: String,
    ) -> Self {
        Self {
            entity_id,
            entity_type,
            path,
        }
    }

    pub fn entity_id(&self) -> &Identifier {
        &self.entity_id
    }

    pub fn entity_type(&self) -> &DocumentPropertyReferenceTarget {
        &self.entity_type
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

impl From<ReferencedEntityNotFoundError> for ConsensusError {
    fn from(err: ReferencedEntityNotFoundError) -> Self {
        Self::StateError(StateError::ReferencedEntityNotFoundError(err))
    }
}
