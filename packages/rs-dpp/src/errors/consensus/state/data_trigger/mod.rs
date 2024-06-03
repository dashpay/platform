use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use crate::identity::state_transition::asset_lock_proof::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

pub mod data_trigger_condition_error;
pub mod data_trigger_execution_error;
pub mod data_trigger_invalid_result_error;

#[derive(
    Error, Debug, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize, Clone,
)]
pub enum DataTriggerError {
    /*

    DO NOT CHANGE ORDER OF VARIANTS WITHOUT INTRODUCING OF NEW VERSION

    */
    #[error(transparent)]
    DataTriggerConditionError(DataTriggerConditionError),

    #[error(transparent)]
    DataTriggerExecutionError(DataTriggerExecutionError),

    #[error(transparent)]
    DataTriggerInvalidResultError(DataTriggerInvalidResultError),
}

impl From<DataTriggerError> for ConsensusError {
    fn from(error: DataTriggerError) -> Self {
        Self::StateError(StateError::DataTriggerError(error))
    }
}
