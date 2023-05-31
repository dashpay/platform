use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Data Contract {data_contract_id} is not present")]
pub struct DataContractNotPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
}

impl DataContractNotPresentError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}

impl From<DataContractNotPresentError> for ConsensusError {
    fn from(err: DataContractNotPresentError) -> Self {
        Self::BasicError(BasicError::DataContractNotPresentError(err))
    }
}
