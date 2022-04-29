use crate::data_contract::DataContract;
use crate::mocks;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataContractError {
    #[error("Data contract already exists")]
    DataContractAlreadyExistsError,

    #[error("Invalid Data Contract: {errors:?}")]
    InvalidDataContractError {
        errors: Vec<mocks::ConsensusError>,
        raw_data_contract: DataContract,
    },

    #[error("Data Contract doesn't define document with typ {doc_type}")]
    InvalidDocumentTypeError {
        doc_type: String,
        data_contract: DataContract,
    },
}
