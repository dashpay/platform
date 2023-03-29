use crate::consensus::basic::BasicError;
use thiserror::Error;
use platform_value::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Data Contract {data_contract_id} is not present")]
pub struct DataContractNotPresentError {
    data_contract_id: Identifier
}

impl DataContractNotPresentError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self {
            data_contract_id
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id.clone()
    }
}

impl From<DataContractNotPresentError> for BasicError {
    fn from(err: DataContractNotPresentError) -> Self {
        Self::DataContractNotPresentError(err)
    }
}
