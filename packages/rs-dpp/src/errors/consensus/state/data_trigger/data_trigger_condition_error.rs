use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("{message}")]
pub struct DataTriggerConditionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_id: Identifier,
    message: String,
}

impl DataTriggerConditionError {
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

impl From<DataTriggerConditionError> for ConsensusError {
    fn from(err: DataTriggerConditionError) -> Self {
        Self::StateError(err.into())
    }
}

impl From<DataTriggerConditionError> for StateError {
    fn from(err: DataTriggerConditionError) -> Self {
        StateError::DataTriggerError(err.into())
    }
}

impl From<DataTriggerConditionError> for DataTriggerError {
    fn from(err: DataTriggerConditionError) -> Self {
        DataTriggerError::DataTriggerConditionError(err)
    }
}
