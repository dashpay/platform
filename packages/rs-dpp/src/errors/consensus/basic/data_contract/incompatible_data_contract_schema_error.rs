use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::document::document_transition::document_base_transition::JsonValue;
use crate::prelude::Identifier;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("Data Contract updated schema is not backward compatible with one defined in Data Contract wid id {data_contract_id}. Field: '{field_path}', Operation: '{operation}'"
)]
pub struct IncompatibleDataContractSchemaError {
    data_contract_id: Identifier,
    operation: String,
    field_path: String,
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
