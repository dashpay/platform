use crate::consensus::basic::value_error::ValueError;
use crate::consensus::state::data_trigger::data_trigger_condition_error::DataTriggerConditionError;
use crate::consensus::state::data_trigger::data_trigger_execution_error::DataTriggerExecutionError;
use crate::consensus::state::data_trigger::data_trigger_invalid_result_error::DataTriggerInvalidResultError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::state_transition_action::document::documents_batch::document_transition::DocumentTransitionAction;
use bincode::{Decode, Encode};
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize, Encode, Decode)]
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

// TODO(v0.24-backport): move these errors to dedicated files?
/// Data trigger errors represent issues that occur while processing data triggers.
/// Data triggers are custom logic associated with the creation, modification, or deletion of documents.
#[derive(Error, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum DataTriggerActionError {
    /// An error occurred while evaluating the condition of the data trigger.
    #[error("{message}")]
    DataTriggerConditionError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// A message describing the error.
        message: String,
        /// The document transition associated with the error, if available.
        #[bincode(with_serde)]
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// An error occurred during the execution of the data trigger.
    #[error("{message}")]
    DataTriggerExecutionError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// A message describing the error.
        message: String,
        /// A message describing the execution error.
        execution_error: String,
        /// The document transition associated with the error, if available.
        #[bincode(with_serde)]
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// The data trigger did not return any result, which is invalid.
    #[error("Data trigger have not returned any result")]
    DataTriggerInvalidResultError {
        /// The identifier of the associated data contract.
        data_contract_id: Identifier,
        /// The identifier of the associated document transition.
        document_transition_id: Identifier,
        /// The document transition associated with the error, if available.
        #[bincode(with_serde)]
        document_transition: Option<DocumentTransitionAction>,
        /// The owner identifier associated with the error, if available.
        owner_id: Option<Identifier>,
    },

    /// A value error occurred while processing the data trigger.
    #[error("value error: {0}")]
    ValueError(#[from] ValueError),
}

#[cfg(feature = "validation")]
impl From<DataTriggerActionError> for StateError {
    fn from(v: DataTriggerActionError) -> Self {
        StateError::DataTriggerActionError(v)
    }
}

#[cfg(feature = "validation")]
impl From<DataTriggerError> for StateError {
    fn from(error: DataTriggerError) -> Self {
        StateError::DataTriggerError(error)
    }
}

#[cfg(feature = "validation")]
impl From<DataTriggerError> for ConsensusError {
    fn from(error: DataTriggerError) -> Self {
        Self::StateError(StateError::DataTriggerError(error))
    }
}
