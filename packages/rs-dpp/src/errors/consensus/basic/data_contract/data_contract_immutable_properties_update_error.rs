use crate::consensus::basic::BasicError;
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Only $defs, version and documents fields are allowed to be updated. Forbidden operation '{operation}' on '{field_path}'")]
pub struct DataContractImmutablePropertiesUpdateError {
    operation: String,
    field_path: String,
}

impl DataContractImmutablePropertiesUpdateError {
    pub fn new(operation: String, field_path: String) -> Self {
        Self {
            operation,
            field_path,
        }
    }

    pub fn operation(&self) -> String {
        self.operation.clone()
    }

    pub fn field_path(&self) -> String {
        self.field_path.clone()
    }
}

impl From<DataContractImmutablePropertiesUpdateError> for ConsensusError {
    fn from(err: DataContractImmutablePropertiesUpdateError) -> Self {
        Self::BasicError(Box::new(
            BasicError::DataContractImmutablePropertiesUpdateError(err),
        ))
    }
}
