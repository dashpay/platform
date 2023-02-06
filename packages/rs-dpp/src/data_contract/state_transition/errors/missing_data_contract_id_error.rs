use crate::consensus::basic::BasicError;
use thiserror::Error;

use crate::document::document_transition::document_base_transition::JsonValue;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("$dataContractId is not present")]
pub struct MissingDataContractIdError {
    raw_document_transition: JsonValue,
}

impl MissingDataContractIdError {
    pub fn new(raw_document_transition: JsonValue) -> Self {
        Self {
            raw_document_transition,
        }
    }

    pub fn raw_document_transition(&self) -> JsonValue {
        self.raw_document_transition.clone()
    }
}

impl From<MissingDataContractIdError> for BasicError {
    fn from(err: MissingDataContractIdError) -> Self {
        Self::MissingDataContractIdError(err)
    }
}
