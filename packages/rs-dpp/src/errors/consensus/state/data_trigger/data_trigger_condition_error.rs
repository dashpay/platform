use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::DocumentTransition;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("{message}")]
pub struct DataTriggerConditionError {
    data_contract_id: Identifier,
    document_transition_id: Identifier,
    message: String,

    #[serde(skip)]
    document_transition: Option<DocumentTransition>,

    #[serde(skip)]
    owner_id: Option<Identifier>,
}

impl DataTriggerConditionError {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
    ) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
            message,
            document_transition: None,
            owner_id: None,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
    pub fn document_transition_id(&self) -> &Identifier {
        &self.document_transition_id
    }
    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn set_document_transition(&mut self, document_transition: DocumentTransition) {
        self.document_transition = Some(document_transition);
    }

    pub fn document_transition(&self) -> &Option<DocumentTransition> {
        &self.document_transition
    }

    pub fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = Some(owner_id);
    }

    pub fn owner_id(&self) -> &Option<Identifier> {
        &self.owner_id
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
