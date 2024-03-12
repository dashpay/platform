use bincode::{Decode, Encode};
use thiserror::Error;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use crate::consensus::basic::decode::DecodingError;

use crate::data_contract::DataContract;

use crate::consensus::basic::document::InvalidDocumentTypeError;
use crate::data_contract::errors::json_schema_error::JsonSchemaError;
use crate::ProtocolError;

// @append_only
#[derive(Error, Debug, PlatformSerialize, PlatformDeserialize, Encode, Decode, Clone)]
pub enum DataContractError {
    #[error("Data Contract already exists")]
    DataContractAlreadyExistsError,

    // #[error("Invalid Data Contract: {errors:?}")]
    // InvalidDataContractError {
    //     errors: Vec<ConsensusError>,
    //     raw_data_contract: DataContract,
    // },


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
    CorruptedCodeExecution(String),

    #[error("Corrupted Code Execution: {0}")]
    JsonSchema(JsonSchemaError),
}
