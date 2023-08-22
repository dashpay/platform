use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::prelude::Identifier;
use serde_json::Value as JsonValue;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("Data Contract updated schema is not backward compatible with one defined in Data Contract wid id {data_contract_id}. Field: '{field_path}', Operation: '{operation}'"
)]
pub struct IncompatibleDataContractSchemaError {
    data_contract_id: Identifier,
    operation: String,
    field_path: String,
    #[bincode(with_serde)]
    old_schema: JsonValue,
    #[bincode(with_serde)]
    new_schema: JsonValue,
}

impl IncompatibleDataContractSchemaError {
    pub fn new(
        data_contract_id: Identifier,
        operation: String,
        field_path: String,
        old_schema: JsonValue,
        new_schema: JsonValue,
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
    pub fn old_schema(&self) -> JsonValue {
        self.old_schema.clone()
    }
    pub fn new_schema(&self) -> JsonValue {
        self.new_schema.clone()
    }
}

impl From<IncompatibleDataContractSchemaError> for ConsensusError {
    fn from(err: IncompatibleDataContractSchemaError) -> Self {
        Self::BasicError(BasicError::IncompatibleDataContractSchemaError(err))
    }
}
