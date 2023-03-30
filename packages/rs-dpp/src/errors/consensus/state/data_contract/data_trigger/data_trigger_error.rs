use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::DocumentTransition;
use platform_value::Identifier;
use thiserror::Error;

// TODO not primitive
#[derive(Error, Debug)]
pub enum DataTriggerError {
    #[error("{message}")]
    DataTriggerConditionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },

    #[error("{message}")]
    DataTriggerExecutionError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        // ? maybe we should replace with source
        execution_error: anyhow::Error,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },

    #[error("Data trigger have not returned any result")]
    DataTriggerInvalidResultError {
        data_contract_id: Identifier,
        document_transition_id: Identifier,

        document_transition: Option<DocumentTransition>,
        owner_id: Option<Identifier>,
    },
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
