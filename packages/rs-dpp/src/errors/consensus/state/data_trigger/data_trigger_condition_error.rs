use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::state::data_trigger::data_trigger_error::DataTriggerActionError;
use crate::document::document_transition::DocumentTransitionAction;
use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[error("{message}")]
pub struct DataTriggerConditionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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
}

impl DataTriggerConditionError {
    pub fn new(
        data_contract_id: Identifier,
        document_transition_id: Identifier,
        message: String,
        document_transition: Option<DocumentTransitionAction>,
        owner_id: Option<Identifier>,
    ) -> Self {
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
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl From<DataTriggerConditionError> for ConsensusError {
    fn from(err: DataTriggerConditionError) -> Self {
        Self::StateError(err.into())
    }
}

impl From<DataTriggerConditionError> for StateError {
    fn from(err: DataTriggerConditionError) -> Self {
        StateError::DataTriggerActionError(err.into())
    }
}

impl From<DataTriggerConditionError> for DataTriggerActionError {
    fn from(err: DataTriggerConditionError) -> Self {
        DataTriggerActionError::DataTriggerConditionError(err)
    }
}
