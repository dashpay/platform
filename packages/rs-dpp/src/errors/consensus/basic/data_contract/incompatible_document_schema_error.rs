use crate::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::prelude::Identifier;

use bincode::{Decode, Encode};
use platform_value::Value;

#[derive(
    Error, Debug, Clone, PartialEq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("New document schema is not backward compatible with existing one. Field: '{field_path}', Operation: '{operation}'"
)]
#[platform_serialize(unversioned)]
pub struct IncompatibleDocumentSchemaError {
    data_contract_id: Identifier,
    operation: String,
    field_path: String,
    old_schema: Value,
    new_schema: Value,
}

impl IncompatibleDocumentSchemaError {
    pub fn new(
        data_contract_id: Identifier,
        operation: String,
        field_path: String,
        old_schema: Value,
        new_schema: Value,
    ) -> Self {
        Self {
            data_contract_id,
            operation,
            field_path,
            old_schema,
            new_schema,
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
    pub fn old_schema(&self) -> Value {
        self.old_schema.clone()
    }
    pub fn new_schema(&self) -> Value {
        self.new_schema.clone()
    }
}

impl From<IncompatibleDocumentSchemaError> for ConsensusError {
    fn from(err: IncompatibleDocumentSchemaError) -> Self {
        Self::BasicError(BasicError::IncompatibleDataContractSchemaError(err))
    }
}
