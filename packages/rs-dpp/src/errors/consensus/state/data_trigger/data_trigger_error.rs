use crate::consensus::basic::value_error::ValueError;
use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Data trigger errors represent issues that occur while processing data triggers.
/// Data triggers are custom logic associated with the creation, modification, or deletion of documents.
#[derive(Error, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum DataTriggerActionError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    /// An error occurred while evaluating the condition of the data trigger.
    #[error(transparent)]
    DataTriggerConditionError(DataTriggerConditionError),

    /// An error occurred during the execution of the data trigger.
    #[error(transparent)]
    DataTriggerExecutionError(DataTriggerExecutionError),

    /// The data trigger did not return any result, which is invalid.
    #[error(transparent)]
    DataTriggerInvalidResultError(DataTriggerInvalidResultError),

    /// A value error occurred while processing the data trigger.
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),
}

impl From<DataTriggerActionError> for StateError {
    fn from(v: DataTriggerActionError) -> Self {
        StateError::DataTriggerActionError(v)
    }
}

impl From<DataTriggerActionError> for ConsensusError {
    fn from(error: DataTriggerActionError) -> Self {
        Self::StateError(StateError::DataTriggerActionError(error))
    }
}
