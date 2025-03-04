use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;
use crate::errors::ProtocolError;
#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "Can't update Document Type {data_contract_id}::{document_type_name}: {additional_message}"
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DocumentTypeUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub data_contract_id: Identifier,
    pub document_type_name: String,
    pub additional_message: String,
}

impl DocumentTypeUpdateError {
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: impl Into<String>,
        additional_message: impl Into<String>,
    ) -> Self {
        Self {
            data_contract_id,
            document_type_name: document_type_name.into(),
            additional_message: additional_message.into(),
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn document_type_name(&self) -> &String {
        &self.document_type_name
    }
    pub fn additional_message(&self) -> &str {
        &self.additional_message
    }
}

impl From<DocumentTypeUpdateError> for ConsensusError {
    fn from(err: DocumentTypeUpdateError) -> Self {
        Self::StateError(StateError::DocumentTypeUpdateError(err))
    }
}
