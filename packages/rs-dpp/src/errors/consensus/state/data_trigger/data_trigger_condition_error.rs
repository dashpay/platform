use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use platform_value::Identifier;
use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::prelude::DocumentTransition;

#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct DataTriggerConditionError {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,

    document_transition: Option<DocumentTransition>,
    owner_id: Option<Identifier>,
}

impl DataTriggerConditionError {
    pub fn new( data_contract_id: Identifier,
                document_transition_id: Identifier,
                message: String,

                document_transition: Option<DocumentTransition>,
                owner_id: Option<Identifier>,) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,

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
    pub fn document_transition(&self) -> &Option<DocumentTransition> {
        &self.document_transition
    }
    pub fn owner_id(&self) -> &Option<Identifier> {
        &self.owner_id
    }
}

impl From<DataTriggerConditionError> for ConsensusError {
    fn from(err: DataTriggerConditionError) -> Self {
        Self::StateError(StateError::DataTriggerError(DataTriggerError::DataTriggerConditionError(err)))
    }
}
