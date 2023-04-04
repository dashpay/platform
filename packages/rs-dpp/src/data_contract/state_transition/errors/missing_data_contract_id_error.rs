use crate::ProtocolError;
use platform_value::Value;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
#[error("$dataContractId is not present")]
pub struct MissingDataContractIdError {
    raw_document_transition: Value,
}

impl MissingDataContractIdError {
    pub fn new(raw_document_transition: Value) -> Self {
        Self {
            raw_document_transition,
        }
    }

    pub fn raw_document_transition(&self) -> Value {
        self.raw_document_transition.clone()
    }
}

impl From<MissingDataContractIdError> for ProtocolError {
    fn from(err: MissingDataContractIdError) -> Self {
        Self::MissingDataContractIdError(err)
    }
}
