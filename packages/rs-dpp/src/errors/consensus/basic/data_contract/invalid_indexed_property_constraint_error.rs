use crate::consensus::basic::{BasicError, IndexError};
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::data_contract::document_type::Index;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Indexed property '{property_name}' for '{document_type}' document has an invalid constraint '{constraint_name}', reason: '{reason}'")]
pub struct InvalidIndexedPropertyConstraintError {
    document_type: String,
    index_definition: Index,
    property_name: String,
    constraint_name: String,
    reason: String,
}

impl InvalidIndexedPropertyConstraintError {
    pub fn new(
        document_type: String,
        index_definition: Index,
        property_name: String,
        constraint_name: String,
        reason: String,
    ) -> Self {
        Self {
            document_type,
            index_definition,
            property_name,
            constraint_name,
            reason,
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
    pub fn constraint_name(&self) -> String {
        self.constraint_name.clone()
    }
    pub fn reason(&self) -> String {
        self.reason.clone()
    }
}

impl From<InvalidIndexedPropertyConstraintError> for ConsensusError {
    fn from(err: InvalidIndexedPropertyConstraintError) -> Self {
        Self::BasicError(BasicError::IndexError(
            BasicError::InvalidIndexedPropertyConstraintError(err),
        ))
    }
}
