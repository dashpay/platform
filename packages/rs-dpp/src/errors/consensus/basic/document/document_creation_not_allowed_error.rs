use crate::errors::consensus::basic::BasicError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::data_contract::document_type::restricted_creation::CreationRestrictionMode;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Document Creation on {data_contract_id}:{document_type_name} is not allowed because of the document type's creation restriction mode {creation_restriction_mode}")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DocumentCreationNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub data_contract_id: Identifier,

    pub document_type_name: String,

    pub creation_restriction_mode: CreationRestrictionMode,
}

impl DocumentCreationNotAllowedError {
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: String,
        creation_restriction_mode: CreationRestrictionMode,
    ) -> Self {
        Self {
            data_contract_id,
            document_type_name,
            creation_restriction_mode,
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn document_type_name(&self) -> &str {
        self.document_type_name.as_str()
    }

    pub fn creation_restriction_mode(&self) -> CreationRestrictionMode {
        self.creation_restriction_mode
    }
}

impl From<DocumentCreationNotAllowedError> for ConsensusError {
    fn from(err: DocumentCreationNotAllowedError) -> Self {
        Self::BasicError(BasicError::DocumentCreationNotAllowedError(err))
    }
}
