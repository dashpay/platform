use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;

// TODO not primitive
#[derive(Error, Debug)]
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
