use thiserror::Error;

use crate::{prelude::*, util::json_schema::Index};

#[derive(Error, Debug)]
pub enum BasicError {
    #[error("Data Contract {data_contract_id} is not present")]
    DataContractNotPresent { data_contract_id: Identifier },

    #[error("Data Contract version must be {expected_version}, go {version}")]
    InvalidDataContractVersionError { expected_version: u32, version: u32 },

    #[error("JSON Schema depth is greater than {0}")]
    DataContractMaxDepthExceedError(usize),

    // Document
    #[error(
        "Data Contract {data_contract_id} doesn't define document with the type {document_type}"
    )]
    InvalidDocumentTypeError {
        document_type: String,
        data_contract_id: Identifier,
    },

    #[error("Duplicate index name '{duplicate_index_name}' defined in '{document_type}' document")]
    DuplicateIndexNameError {
        document_type: String,
        duplicate_index_name: String,
    },

    #[error("Invalid JSON Schema $ref: {ref_error}")]
    InvalidJsonSchemaRefError { ref_error: String },

    #[error(transparent)]
    IndexError(IndexError),

    #[error("{0}")]
    JsonSchemaCompilationError(String),

    #[error(
        "Unique compound index properties {:?} are partially set for {document_type}",
        index_properties
    )]
    InconsistentCompoundIndexDataError {
        index_properties: Vec<String>,
        document_type: String,
    },

    #[error("$type is not present")]
    MissingDocumentTypeError,

    #[error("$action is not present")]
    MissingDocumentTransitionActionError,

    #[error("Document transition action {} is not supported", action)]
    InvalidDocumentTransitionActionError { action: String },

    #[error(
        "Invalid document transition id {}, expected {}",
        invalid_id,
        expected_id
    )]
    InvalidDocumentTransitionIdError {
        expected_id: Identifier,
        invalid_id: Identifier,
    },

    #[error("Document transitions with duplicate IDs {:?}", references)]
    DuplicateDocumentTransitionsWithIdsError { references: Vec<(String, Vec<u8>)> },

    #[error("$dataContractId is not present")]
    MissingDataContractIdError,

    #[error("Invalid {}: {}", identifier_name, error)]
    InvalidIdentifierError {
        identifier_name: String,
        error: String,
    },

    #[error("Document with type {document_type} has updated unique index named '{index_name}'. Change of unique indices is not allowed")]
    DataContractUniqueIndicesChangedError {
        document_type: String,
        index_name: String,
    },

    #[error("Document with type {document_type} has badly constructed index '{index_name}'. Existing properties in the indices should be defined in the beginning of it. ")]
    DataContractInvalidIndexDefinitionUpdateError {
        document_type: String,
        index_name: String,
    },

    #[error("Document with type {document_type} has a new unique index named '{index_name}'. Adding unique indices during Data Contract update is not allowed.")]
    DataContractHaveNewUniqueIndexError {
        document_type: String,
        index_name: String,
    },

    #[error("Identity {identity_id} not found")]
    IdentityNotFoundError { identity_id: Identifier },
}

impl From<IndexError> for BasicError {
    fn from(error: IndexError) -> Self {
        BasicError::IndexError(error)
    }
}

#[derive(Error, Debug)]
pub enum IndexError {
    #[error("'{document_type}' document has more than '{index_limit}' unique indexes")]
    UniqueIndicesLimitReachedError {
        document_type: String,
        index_limit: usize,
    },

    #[error("System property '{property_name}' is already indexed and can't be used in other indices for '{document_type}' document")]
    SystemPropertyIndexAlreadyPresentError {
        document_type: String,
        index_definition: Index,
        property_name: String,
    },

    #[error("'{property_name}' property is not defined in the '{document_type}' document")]
    UndefinedIndexPropertyError {
        document_type: String,
        index_definition: Index,
        property_name: String,
    },

    #[error("'{property_name}' property ofr '{document_type}' document has an invalid type '{property_type}' and cannot be use as an index")]
    InvalidIndexPropertyTypeError {
        document_type: String,
        index_definition: Index,
        property_name: String,
        property_type: String,
    },

    #[error("Indexed property '{property_name}' for '{document_type}' document has an invalid constraint '{constraint_name}', reason: '{reason}'")]
    InvalidIndexedPropertyConstraintError {
        document_type: String,
        index_definition: Index,
        property_name: String,
        constraint_name: String,
        reason: String,
    },

    #[error(
        "All or none of unique compound properties must be set for '{document_type}' document"
    )]
    InvalidCompoundIndexError {
        document_type: String,
        index_definition: Index,
    },

    #[error("Duplicate index definition for '{document_type} document")]
    DuplicateIndexError {
        document_type: String,
        index_definition: Index,
    },
}
