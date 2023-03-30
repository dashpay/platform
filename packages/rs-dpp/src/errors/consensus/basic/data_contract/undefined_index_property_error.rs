use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[error("'{property_name}' property is not defined in the '{document_type}' document")]
pub struct UndefinedIndexPropertyError {
    document_type: String,
    index_name: String,
    property_name: String,
}

impl UndefinedIndexPropertyError {
    pub fn new(document_type: String, index_name: String, property_name: String) -> Self {
        Self {
            document_type,
            index_name,
            property_name,
        }
    }

    pub fn document_type(&self) -> &str {
        &self.document_type
    }
    pub fn index_definition(&self) -> &str {
        &self.index_name
    }
    pub fn property_name(&self) -> &str {
        &self.property_name
    }
}

impl From<UndefinedIndexPropertyError> for ConsensusError {
    fn from(err: UndefinedIndexPropertyError) -> Self {
        Self::BasicError(BasicError::UndefinedIndexPropertyError(err))
    }
}
