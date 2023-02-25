use crate::consensus::ConsensusError;
use thiserror::Error;

use crate::document::document_transition::document_base_transition::JsonValue;
use crate::ProtocolError;

#[derive(Error, Debug)]
#[error("Invalid Data Contract: {errors:?}")]
pub struct InvalidDataContractError {
    pub errors: Vec<ConsensusError>,
    raw_data_contract: JsonValue,
}

impl InvalidDataContractError {
    pub fn new(errors: Vec<ConsensusError>, raw_data_contract: JsonValue) -> Self {
        Self {
            errors,
            raw_data_contract,
        }
    }

    pub fn errors(&self) -> &[ConsensusError] {
        &self.errors
    }
    pub fn raw_data_contract(&self) -> JsonValue {
        self.raw_data_contract.clone()
    }
}

impl From<InvalidDataContractError> for ProtocolError {
    fn from(err: InvalidDataContractError) -> Self {
        Self::InvalidDataContractError(err)
    }
}
