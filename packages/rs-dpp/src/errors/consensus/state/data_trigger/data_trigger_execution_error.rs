use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::state::data_trigger::DataTriggerError;
use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[error("{message}")]
pub struct DataTriggerExecutionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_id: Identifier,
    message: String,
}

impl DataTriggerExecutionError {
    pub fn new(data_contract_id: Identifier, document_id: Identifier, message: String) -> Self {
        Self {
            data_contract_id,
            document_id,
            message,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }

    pub fn document_id(&self) -> &Identifier {
        &self.document_id
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<DataTriggerExecutionError> for ConsensusError {
    fn from(err: DataTriggerExecutionError) -> Self {
        Self::StateError(err.into())
    }
}

impl From<DataTriggerExecutionError> for StateError {
    fn from(err: DataTriggerExecutionError) -> Self {
        StateError::DataTriggerError(DataTriggerError::DataTriggerExecutionError(err))
    }
}
