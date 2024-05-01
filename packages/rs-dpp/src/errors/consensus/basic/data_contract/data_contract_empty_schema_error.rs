use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} schema is empty.")]
#[platform_serialize(unversioned)]
pub struct DataContractEmptySchemaError {
    data_contract_id: Identifier,
}

impl DataContractEmptySchemaError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}

impl From<DataContractEmptySchemaError> for ConsensusError {
    fn from(err: DataContractEmptySchemaError) -> Self {
        Self::BasicError(BasicError::DataContractEmptySchemaError(err))
    }
}
