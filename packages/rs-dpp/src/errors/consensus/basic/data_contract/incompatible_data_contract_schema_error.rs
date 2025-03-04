use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};
use platform_value::Identifier;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Data Contract updated schema is not backward compatible with one defined in Data Contract with id {data_contract_id}. Field: '{field_path}', Operation: '{operation}'"
)]
#[platform_serialize(unversioned)]
#[cfg_attr(feature = "apple", ferment_macro::export)]
pub struct IncompatibleDataContractSchemaError {
    pub data_contract_id: Identifier,
    pub operation: String,
    pub field_path: String,
}

impl IncompatibleDataContractSchemaError {
    pub fn new(data_contract_id: Identifier, operation: String, field_path: String) -> Self {
        Self {
            data_contract_id,
            operation,
            field_path,
        }
    }

    pub fn data_contract_id(&self) -> Identifier {
        self.data_contract_id
    }
    pub fn operation(&self) -> String {
        self.operation.clone()
    }
    pub fn field_path(&self) -> String {
        self.field_path.clone()
    }
}

impl From<IncompatibleDataContractSchemaError> for ConsensusError {
    fn from(err: IncompatibleDataContractSchemaError) -> Self {
        Self::BasicError(BasicError::IncompatibleDataContractSchemaError(err))
    }
}
