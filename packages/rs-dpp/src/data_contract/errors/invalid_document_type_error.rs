use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::DataContract;

#[derive(Error, Debug, Clone, PartialEq)]
#[error("Data Contract doesn't define document with type {doc_type}")]
pub struct InvalidDataContractError {
    doc_type: String,
    data_contract: DataContract,
}

impl InvalidDataContractError {
    pub fn new(doc_type: String, data_contract: DataContract) -> Self {
        Self {
            doc_type,
            data_contract,
        }
    }

    pub fn doc_type(&self) -> String {
        self.doc_type.clone()
    }
    pub fn data_contract(&self) -> DataContract {
        self.data_contract.clone()
    }
}

impl From<InvalidDataContractError> for ConsensusError {
    fn from(err: InvalidDataContractError) -> Self {
        Self::InvalidDataContractError(err)
    }
}
