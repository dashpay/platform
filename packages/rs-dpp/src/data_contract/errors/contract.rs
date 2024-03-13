use crate::consensus::basic::decode::DecodingError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::errors::json_schema_error::JsonSchemaError;
use crate::ProtocolError;

// @append_only
#[derive(Error, Debug, PlatformSerialize, PlatformDeserialize, Encode, Decode, Clone)]
pub enum DataContractError {
    #[error(transparent)]
    DecodingContractError(DecodingError),

    #[error(transparent)]
    DecodingDocumentError(DecodingError),

    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error("missing required key: {0}")]
    MissingRequiredKey(String),

    #[error("field requirement unmet: {0}")]
    FieldRequirementUnmet(String),

    #[error("key wrong type error: {0}")]
    KeyWrongType(String),

    #[error("value wrong type error: {0}")]
    ValueWrongType(String),

    #[error("invalid uri error: {0}")]
    InvalidURI(String),

    /// Key wrong bounds error
    #[error("key out of bounds error: {0}")]
    KeyWrongBounds(String),

    /// A key value pair must exist
    #[error("key value must exist: {0}")]
    KeyValueMustExist(String),

    #[error("value decoding error: {0}")]
    ValueDecodingError(String),

    #[error("encoding data structure not supported error: {0}")]
    EncodingDataStructureNotSupported(String),

    #[error("invalid contract structure: {0}")]
    InvalidContractStructure(String),

    #[error("document type not found: {0}")]
    DocumentTypeNotFound(String),

    #[error("document type field not found: {0}")]
    DocumentTypeFieldNotFound(String),

    #[error("reference definition not found error: {0}")]
    ReferenceDefinitionNotFound(String),

    #[error("document owner id missing error: {0}")]
    DocumentOwnerIdMissing(String),

    #[error("document id missing error: {0}")]
    DocumentIdMissing(String),

    #[error("Operation not supported: {0}")]
    Unsupported(String),

    #[error("Corrupted Serialization: {0}")]
    CorruptedSerialization(String),

    #[error("Corrupted Code Execution: {0}")]
    JsonSchema(JsonSchemaError),
}

impl From<platform_value::Error> for DataContractError {
    fn from(value: platform_value::Error) -> Self {
        DataContractError::ValueDecodingError(format!("{:?}", value))
    }
}

impl From<(platform_value::Error, &str)> for DataContractError {
    fn from(value: (platform_value::Error, &str)) -> Self {
        DataContractError::ValueDecodingError(format!("{}: {:?}", value.1, value.0))
    }
}
