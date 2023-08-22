use crate::consensus::ConsensusError;
use crate::ProtocolError;
use platform_value::Value;
use thiserror::Error;

// @append_only
#[derive(Error, Debug)]
#[error("Invalid Data Contract: {errors:?}")]
pub struct InvalidDataContractError {
    pub errors: Vec<ConsensusError>,
    raw_data_contract: Value,
}

impl InvalidDataContractError {
    pub fn new(errors: Vec<ConsensusError>, raw_data_contract: Value) -> Self {
        Self {
            errors,
            raw_data_contract,
        }
    }

    pub fn errors(&self) -> &[ConsensusError] {
        &self.errors
    }
    pub fn raw_data_contract(&self) -> Value {
        self.raw_data_contract.clone()
    }
}

impl From<InvalidDataContractError> for ProtocolError {
    fn from(err: InvalidDataContractError) -> Self {
        Self::InvalidDataContractError(err)
    }
}
