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
#[error("Data Contract {data_contract_id} is already present")]
#[platform_serialize(unversioned)]
pub struct DataContractAlreadyPresentError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    data_contract_id: Identifier,
}

impl DataContractAlreadyPresentError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
}

impl From<DataContractAlreadyPresentError> for ConsensusError {
    fn from(err: DataContractAlreadyPresentError) -> Self {
        Self::StateError(StateError::DataContractAlreadyPresentError(err))
    }
}
