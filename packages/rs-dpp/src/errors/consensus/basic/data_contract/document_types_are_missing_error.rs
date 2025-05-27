use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;
use platform_value::Identifier;

use crate::data_contract::errors::contract::DataContractError;
use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract {data_contract_id} must have at least one document type or token defined.")]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct DocumentTypesAreMissingError {
    pub data_contract_id: Identifier,
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
