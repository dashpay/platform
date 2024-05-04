use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Updated document type '{document_type_name}' schema is not backward compatible with previous version. Incompatible change '{operation}' of property '{property_path}'"
)]
#[platform_serialize(unversioned)]
pub struct IncompatibleDocumentTypeSchemaError {
    document_type_name: String,
    operation: String,
    property_path: String,
}

impl IncompatibleDocumentTypeSchemaError {
    pub fn new(
        document_type_name: impl Into<String>,
        operation: impl Into<String>,
        property_path: impl Into<String>,
    ) -> Self {
        Self {
            document_type_name: document_type_name.into(),
            operation: operation.into(),
            property_path: property_path.into(),
        }
    }

    pub fn document_type_name(&self) -> &str {
        &self.document_type_name
    }
    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn property_path(&self) -> &str {
        &self.property_path
    }
}

impl From<IncompatibleDocumentTypeSchemaError> for ConsensusError {
    fn from(err: IncompatibleDocumentTypeSchemaError) -> Self {
        Self::BasicError(BasicError::IncompatibleDocumentTypeSchemaError(err))
    }
}
