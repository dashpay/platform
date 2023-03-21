use crate::consensus::basic::IndexError;
use thiserror::Error;

use crate::data_contract::document_type::Index;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("System property '{property_name}' is already indexed and can't be used in other indices for '{document_type}' document")]
pub struct SystemPropertyIndexAlreadyPresentError {
    document_type: String,
    index_definition: Index,
    property_name: String,
}

impl SystemPropertyIndexAlreadyPresentError {
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

impl From<SystemPropertyIndexAlreadyPresentError> for IndexError {
    fn from(err: SystemPropertyIndexAlreadyPresentError) -> Self {
        Self::SystemPropertyIndexAlreadyPresentError(err)
    }
}
