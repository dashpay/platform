use crate::consensus::basic::{BasicError, IndexError};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::document_type::Index;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("'{property_name}' property of '{document_type}' document has an invalid type '{property_type}' and cannot be use as an index")]
pub struct InvalidIndexPropertyTypeError {
    document_type: String,
    index_definition: Index,
    property_name: String,
    property_type: String,
}

impl InvalidIndexPropertyTypeError {
    pub fn new(
        document_type: String,
        index_definition: Index,
        property_name: String,
        property_type: String,
    ) -> Self {
        Self {
            document_type,
            index_definition,
            property_name,
            property_type,
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
    pub fn property_type(&self) -> String {
        self.property_type.clone()
    }
}

impl From<InvalidIndexPropertyTypeError> for ConsensusError {
    fn from(err: InvalidIndexPropertyTypeError) -> Self {
        Self::BasicError(Box::new(BasicError::IndexError(
            IndexError::InvalidIndexPropertyTypeError(err),
        )))
    }
}
