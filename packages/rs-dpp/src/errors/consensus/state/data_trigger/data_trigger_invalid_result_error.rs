use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::prelude::DocumentTransition;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("Data trigger have not returned any result")]
pub struct DataTriggerInvalidResultError {
    data_contract_id: Identifier,
    document_transition_id: Identifier,

    #[serde(skip)]
    document_transition: Option<DocumentTransition>,

    #[serde(skip)]
    owner_id: Option<Identifier>,
}

impl DataTriggerInvalidResultError {
    pub fn new(data_contract_id: Identifier, document_transition_id: Identifier) -> Self {
        Self {
            data_contract_id,
            document_transition_id,
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
    pub fn document_transition(&self) -> &Option<DocumentTransition> {
        &self.document_transition
    }
    pub fn owner_id(&self) -> &Option<Identifier> {
        &self.owner_id
    }
    pub fn set_document_transition(&mut self, document_transition: DocumentTransition) {
        self.document_transition = Some(document_transition);
    }
    pub fn set_owner_id(&mut self, owner_id: Identifier) {
        self.owner_id = Some(owner_id);
    }
}

impl From<DataTriggerInvalidResultError> for ConsensusError {
    fn from(err: DataTriggerInvalidResultError) -> Self {
        Self::StateError(err.into())
    }
}

impl From<DataTriggerInvalidResultError> for StateError {
    fn from(err: DataTriggerInvalidResultError) -> Self {
        StateError::DataTriggerError(DataTriggerError::DataTriggerInvalidResultError(err))
    }
}
