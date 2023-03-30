use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerError;
use crate::consensus::ConsensusError;
use thiserror::Error;
use platform_value::Identifier;
use crate::consensus::state::state_error::StateError;
use crate::prelude::DocumentTransition;

#[derive(Error, Debug, Clone)]
#[error("Data trigger have not returned any result")]
pub struct DataTriggerInvalidResultError {
    data_contract_id: Identifier,
    document_transition_id: Identifier,

    document_transition: Option<DocumentTransition>,
    owner_id: Option<Identifier>,
}

impl DataTriggerInvalidResultError {
    pub fn new(data_contract_id: Identifier,
               document_transition_id: Identifier,

               document_transition: Option<DocumentTransition>,
               owner_id: Option<Identifier>,) -> Self {
        Self {
            data_contract_id,
            document_transition_id,

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
    pub fn document_transition(&self) -> &Option<DocumentTransition> {
        &self.document_transition
    }
    pub fn owner_id(&self) -> &Option<Identifier> {
        &self.owner_id
    }
}

impl From<DataTriggerInvalidResultError> for ConsensusError {
    fn from(err: DataTriggerInvalidResultError) -> Self {
        Self::StateError(StateError::DataTriggerError(DataTriggerError::DataTriggerInvalidResultError(err)))
    }
}
