use crate::errors::consensus::state::state_error::StateError;
use crate::errors::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use platform_value::Identifier;
use thiserror::Error;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[platform_serialize(unversioned)]
#[error("Data contract not found for id: {data_contract_id}")]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DataContractNotFoundError {
    pub data_contract_id: Identifier,
}

impl DataContractNotFoundError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> &Identifier {
        &self.data_contract_id
    }
}

impl From<DataContractNotFoundError> for ConsensusError {
    fn from(err: DataContractNotFoundError) -> Self {
        Self::StateError(StateError::DataContractNotFoundError(err))
    }
}
