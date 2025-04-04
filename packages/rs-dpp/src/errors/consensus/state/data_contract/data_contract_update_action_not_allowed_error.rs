use crate::consensus::state::state_error::StateError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Action '{action}' is not allowed on Data Contract {data_contract_id}")]
#[platform_serialize(unversioned)]
pub struct DataContractUpdateActionNotAllowedError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    action: String,
}

impl DataContractUpdateActionNotAllowedError {
    pub fn new(data_contract_id: Identifier, action: impl Into<String>) -> Self {
        Self {
            data_contract_id,
            action: action.into(),
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }

    pub fn action(&self) -> &str {
        &self.action
    }
}

impl From<DataContractUpdateActionNotAllowedError> for ConsensusError {
    fn from(err: DataContractUpdateActionNotAllowedError) -> Self {
        Self::StateError(StateError::DataContractUpdateActionNotAllowedError(err))
    }
}
