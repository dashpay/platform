use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Can't update Data Contract {data_contract_id} config: {additional_message}")]
pub struct DataContractConfigUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    additional_message: String,
}

impl DataContractConfigUpdateError {
    pub fn new(data_contract_id: Identifier, additional_message: impl Into<String>) -> Self {
        Self {
            data_contract_id,
            additional_message: additional_message.into(),
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
    pub fn additional_message(&self) -> &str {
        &self.additional_message
    }
}

impl From<DataContractConfigUpdateError> for ConsensusError {
    fn from(err: DataContractConfigUpdateError) -> Self {
        Self::StateError(StateError::DataContractConfigUpdateError(err))
    }
}
