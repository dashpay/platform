use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

use crate::errors::consensus::state::data_trigger::DataTriggerError;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data trigger have not returned any result")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct DataTriggerInvalidResultError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub data_contract_id: Identifier,
    pub document_id: Identifier,
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
