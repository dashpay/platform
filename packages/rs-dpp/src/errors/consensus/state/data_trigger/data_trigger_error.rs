use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum DataTriggerError {
    #[error(transparent)]
    DataTriggerConditionError(DataTriggerConditionError),

    #[error(transparent)]
    DataTriggerExecutionError(DataTriggerExecutionError),

    #[error(transparent)]
    DataTriggerInvalidResultError(DataTriggerInvalidResultError),
}

impl From<DataTriggerError> for StateError {
    fn from(error: DataTriggerError) -> Self {
        StateError::DataTriggerError(error)
    }
}

impl From<DataTriggerError> for ConsensusError {
    fn from(error: DataTriggerError) -> Self {
        Self::StateError(StateError::DataTriggerError(error))
    }
}
