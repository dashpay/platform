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
#[error(
    "Identity {identity_id} doesn't have permissions to update Data Contract {data_contract_id}"
)]
#[platform_serialize(unversioned)]
pub struct DataContractUpdatePermissionError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
    identity_id: Identifier,
}

impl DataContractUpdatePermissionError {
    pub fn new(data_contract_id: Identifier, identity_id: Identifier) -> Self {
        Self {
            data_contract_id,
            identity_id,
        }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

impl From<DataContractUpdatePermissionError> for ConsensusError {
    fn from(err: DataContractUpdatePermissionError) -> Self {
        Self::StateError(StateError::DataContractUpdatePermissionError(err))
    }
}
