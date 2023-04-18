use crate::consensus::basic::BasicError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, Serialize, Deserialize)]
#[error("only $defs, version and documents fields are allowed to be updated. Forbidden operation '{operation}' on '{field_path}'")]
pub struct DataContractImmutablePropertiesUpdateError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
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

    pub fn operation(&self) -> &str {
        &self.operation
    }

    pub fn field_path(&self) -> &str {
        &self.field_path
    }
}

impl From<DataContractImmutablePropertiesUpdateError> for ConsensusError {
    fn from(err: DataContractImmutablePropertiesUpdateError) -> Self {
        Self::BasicError(BasicError::DataContractImmutablePropertiesUpdateError(err))
    }
}
