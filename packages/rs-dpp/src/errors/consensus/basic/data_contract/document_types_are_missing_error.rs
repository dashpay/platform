use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

use crate::data_contract::errors::DataContractError;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} must have at least one document type defined.")]
#[platform_serialize(unversioned)]
pub struct DocumentTypesAreMissingError {
    data_contract_id: Identifier,
}

impl DocumentTypesAreMissingError {
    pub fn new(data_contract_id: Identifier) -> Self {
        Self { data_contract_id }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
}

impl From<DocumentTypesAreMissingError> for ConsensusError {
    fn from(err: DocumentTypesAreMissingError) -> Self {
        Self::BasicError(BasicError::ContractError(
            DataContractError::DocumentTypesAreMissingError(err),
        ))
    }
}

impl From<DocumentTypesAreMissingError> for DataContractError {
    fn from(err: DocumentTypesAreMissingError) -> Self {
        DataContractError::DocumentTypesAreMissingError(err)
    }
}
