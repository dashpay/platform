use thiserror::Error;

use crate::prelude::*;

#[derive(Error, Debug)]
pub enum BasicError {
    #[error("Data Contract {data_contract_id} is not present")]
    DataContractContPresent { data_contract_id: Identifier },

    #[error("$type is not present")]
    MissingDocumentTypeError,

    #[error("Data Contract version must be {expected_version}, go {version}")]
    InvalidDataContractVersionError { expected_version: u32, version: u32 },

    // Document
    #[error(
        "Data Contract {data_contract_id} doesn't define document with the type {document_type}"
    )]
    InvalidDocumentTypeError {
        document_type: String,
        data_contract_id: Identifier,
    },
}
