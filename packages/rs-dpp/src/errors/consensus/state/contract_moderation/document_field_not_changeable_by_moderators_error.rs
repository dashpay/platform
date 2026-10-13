use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_value::Identifier;
use thiserror::Error;

/// A moderator's field change names a field the document type does not list under
/// `moderatorAbilities.changeFields`; a type listing none refuses every field.
#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error(
    "Field {} of documents of type {} on contract {} can not be changed by moderators",
    field,
    document_type_name,
    contract_id
)]
#[platform_serialize(unversioned)]
pub struct DocumentFieldNotChangeableByModeratorsError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    contract_id: Identifier,
    document_type_name: String,
    field: String,
}

impl DocumentFieldNotChangeableByModeratorsError {
    pub fn new(contract_id: Identifier, document_type_name: String, field: String) -> Self {
        Self {
            contract_id,
            document_type_name,
            field,
        }
    }

    pub fn contract_id(&self) -> Identifier {
        self.contract_id
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }

    pub fn field(&self) -> &str {
        &self.field
    }
}

impl From<DocumentFieldNotChangeableByModeratorsError> for ConsensusError {
    fn from(err: DocumentFieldNotChangeableByModeratorsError) -> Self {
        Self::StateError(StateError::DocumentFieldNotChangeableByModeratorsError(err))
    }
}
