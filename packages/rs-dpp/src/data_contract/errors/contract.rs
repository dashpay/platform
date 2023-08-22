use thiserror::Error;

use crate::data_contract::DataContract;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::errors::json_schema_error::JsonSchemaError;
use crate::errors::consensus::ConsensusError;

// @append_only
#[derive(Error, Debug)]
pub enum DataContractError {
    #[error("Data Contract already exists")]
    DataContractAlreadyExistsError,

    #[error("Invalid Data Contract: {errors:?}")]
    InvalidDataContractError {
        errors: Vec<ConsensusError>,
        raw_data_contract: DataContract,
    },

    #[error(transparent)]
    InvalidDocumentTypeError(InvalidDocumentTypeError),

    #[error("missing required key: {0}")]
    MissingRequiredKey(String),

    #[error("field requirement unmet: {0}")]
    FieldRequirementUnmet(&'static str),

    #[error("key wrong type error: {0}")]
    KeyWrongType(&'static str),

    #[error("value wrong type error: {0}")]
    ValueWrongType(&'static str),

    #[error("value decoding error: {0}")]
    ValueDecodingError(&'static str),

    #[error("encoding data structure not supported error: {0}")]
    EncodingDataStructureNotSupported(&'static str),

    #[error("invalid contract structure: {0}")]
    InvalidContractStructure(String),

    #[error("document type not found: {0}")]
    DocumentTypeNotFound(&'static str),

    #[error("document type field not found: {0}")]
    DocumentTypeFieldNotFound(String),

    #[error("reference definition not found error: {0}")]
    ReferenceDefinitionNotFound(&'static str),

    #[error("document owner id missing error: {0}")]
    DocumentOwnerIdMissing(&'static str),

    #[error("document id missing error: {0}")]
    DocumentIdMissing(&'static str),

    #[error("Operation not supported: {0}")]
    Unsupported(&'static str),

    #[error("Corrupted Serialization: {0}")]
    CorruptedSerialization(&'static str),

    #[error("Corrupted Code Execution: {0}")]
    CorruptedCodeExecution(&'static str),

    #[error("Corrupted Code Execution: {0}")]
    JsonSchema(JsonSchemaError),
}
