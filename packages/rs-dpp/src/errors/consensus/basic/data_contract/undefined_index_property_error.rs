use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use thiserror::Error;

use crate::data_contract::document_type::Index;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("'{property_name}' property is not defined in the '{document_type}' document")]
pub struct UndefinedIndexPropertyError {
    document_type: String,
    index_definition: Index,
    property_name: String,
}

impl UndefinedIndexPropertyError {
    pub fn new(document_type: String, index_definition: Index, property_name: String) -> Self {
        Self {
            document_type,
            index_definition,
            property_name,
        }
    }

    pub fn document_type(&self) -> String {
        self.document_type.clone()
    }
    pub fn index_definition(&self) -> Index {
        self.index_definition.clone()
    }
    pub fn property_name(&self) -> String {
        self.property_name.clone()
    }
}

impl From<UndefinedIndexPropertyError> for ConsensusError {
    fn from(err: UndefinedIndexPropertyError) -> Self {
        Self::BasicError(BasicError::UndefinedIndexPropertyError(err))
    }
}
