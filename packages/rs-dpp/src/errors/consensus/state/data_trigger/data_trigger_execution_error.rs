use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use platform_value::Identifier;
use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::prelude::DocumentTransition;

#[derive(Error, Debug)]
#[error("{message}")]
pub struct DataTriggerExecutionError {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,
    // ? maybe we should replace with source
    execution_error: anyhow::Error,

    document_transition: Option<DocumentTransition>,
    owner_id: Option<Identifier>,
}

impl DataTriggerExecutionError {
    pub fn new(data_contract_id: Identifier,
               document_transition_id: Identifier,
               message: String,
               execution_error: anyhow::Error,

               document_transition: Option<DocumentTransition>,
               owner_id: Option<Identifier>,) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            execution_error,

            document_transition,
            owner_id,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
    pub fn document_transition_id(&self) -> &Identifier {
        &self.document_transition_id
    }
    pub fn message(&self) -> &String {
        &self.message
    }
    pub fn execution_error(&self) -> &anyhow::Error {
        &self.execution_error
    }
    pub fn document_transition(&self) -> &Option<DocumentTransition> {
        &self.document_transition
    }
    pub fn owner_id(&self) -> &Option<Identifier> {
        &self.owner_id
    }

}

impl From<DataTriggerExecutionError> for ConsensusError {
    fn from(err: DataTriggerExecutionError) -> Self {
        Self::StateError(StateError::DataTriggerError(DataTriggerError::DataTriggerExecutionError(err)))
    }
}
