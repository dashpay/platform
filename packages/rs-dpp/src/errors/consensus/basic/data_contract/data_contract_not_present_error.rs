use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract {data_contract_id} is not present")]
pub struct DataContractNotPresentError {
    data_contract_id: Identifier,
}

impl DataContractNotPresentError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id.clone()
    }
}

impl From<DataContractNotPresentError> for ConsensusError {
    fn from(err: DataContractNotPresentError) -> Self {
        Self::BasicError(BasicError::DataContractNotPresentError(err))
    }
}
