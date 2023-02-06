use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::document::document_transition::document_base_transition::JsonValue;

#[derive(Error, Debug)]
#[error("Invalid Data Contract: {errors:?}")]
pub struct InvalidDataContractError {
    errors: Vec<ConsensusError>,
    raw_data_contract: JsonValue,
}

impl InvalidDataContractError {
    pub fn new(errors: Vec<ConsensusError>, raw_data_contract: JsonValue) -> Self {
        Self {
            errors,
            raw_data_contract,
        }
    }

    pub fn errors(&self) -> Vec<ConsensusError> {
        self.errors.clone()
    }
    pub fn raw_data_contract(&self) -> JsonValue {
        self.raw_data_contract.clone()
    }
}

impl From<InvalidDataContractError> for ConsensusError {
    fn from(err: InvalidDataContractError) -> Self {
        Self::InvalidDataContractError(err)
    }
}
