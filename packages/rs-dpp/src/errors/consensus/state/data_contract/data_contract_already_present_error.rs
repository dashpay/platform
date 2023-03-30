use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use platform_value::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract {data_contract_id} is already present")]
pub struct DataContractAlreadyPresentError {
    data_contract_id: Identifier
}

impl DataContractAlreadyPresentError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self {
            data_contract_id,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
}

impl From<DataContractAlreadyPresentError> for ConsensusError {
    fn from(err: DataContractAlreadyPresentError) -> Self {
        Self::StateError(StateError::DataContractAlreadyPresentError(err))
    }
}
