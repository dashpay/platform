use crate::consensus::basic::BasicError;
use platform_value::Value;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("only $defs, version and documents fields are allowed to be updated. Forbidden operation '{operation}' on '{field_path}' old value is '{old_value}', new value is '{new_value}'")]
pub struct DataContractImmutablePropertiesUpdateError {
    operation: String,
    field_path: String,
    old_value: Value,
    new_value: Value,
}

impl DataContractImmutablePropertiesUpdateError {
    pub fn new(operation: String, field_path: String, old_value: Value, new_value: Value) -> Self {
        Self {
            operation,
            field_path,
            old_value,
            new_value,
        }
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }

    pub fn old_value(&self) -> &Value {
        &self.old_value
    }

    pub fn new_value(&self) -> &Value {
        &self.new_value
    }
}

impl From<DataContractImmutablePropertiesUpdateError> for ConsensusError {
    fn from(err: DataContractImmutablePropertiesUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractImmutablePropertiesUpdateError(err))
    }
}
