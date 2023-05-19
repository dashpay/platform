use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Can't update document schemas in the Data Contract {data_contract_id}: Data Contract is readonly")]
pub struct DataContractIsReadonlyError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
}

impl DataContractIsReadonlyError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
}

impl From<DataContractIsReadonlyError> for ConsensusError {
    fn from(err: DataContractIsReadonlyError) -> Self {
        Self::StateError(StateError::DataContractIsReadonlyError(err))
    }
}
