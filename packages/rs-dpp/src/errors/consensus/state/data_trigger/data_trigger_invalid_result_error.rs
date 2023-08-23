use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use platform_value::Identifier;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::state::data_trigger::DataTriggerError;
use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, Serialize, Deserialize, Encode, Decode)]
#[error("Data trigger have not returned any result")]
pub struct DataTriggerInvalidResultError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    document_id: Identifier,
}

impl DataTriggerInvalidResultError {
    pub fn new(data_contract_id: Identifier, document_id: Identifier) -> Self {
        Self {
            data_contract_id,
            document_id,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
    pub fn document_id(&self) -> &Identifier {
        &self.document_id
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
