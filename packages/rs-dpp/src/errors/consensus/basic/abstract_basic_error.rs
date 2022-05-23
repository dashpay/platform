use crate::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BasicError {
    // Document errors
    #[error("Data Contract {data_contract_id} is not present")]
    DataContractContPresent { data_contract_id: Identifier },

    // Data Contract errors
    #[error("Data Contract version must be {expected_version}, go {version}")]
    InvalidDataContractVersionError { expected_version: u32, version: u32 },
}
