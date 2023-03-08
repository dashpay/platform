use crate::consensus::basic::BasicError;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Document transition action {} is not supported", action)]
pub struct InvalidDocumentTransitionActionError {
    action: String,
}

impl InvalidDocumentTransitionActionError {
    pub fn new(action: String) -> Self {
        Self { action }
    }

    pub fn action(&self) -> String {
        self.action.clone()
    }
}

impl From<InvalidDocumentTransitionActionError> for BasicError {
    fn from(err: InvalidDocumentTransitionActionError) -> Self {
        Self::InvalidDocumentTransitionActionError(err)
    }
}
