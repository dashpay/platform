use crate::consensus::basic::BasicError;
use crate::prelude::Identifier;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error(
    "Invalid document transition id {}, expected {}",
    invalid_id,
    expected_id
)]
pub struct InvalidDocumentTransitionIdError {
    expected_id: Identifier,
    invalid_id: Identifier,
}

impl InvalidDocumentTransitionIdError {
    pub fn new(expected_id: Identifier, invalid_id: Identifier) -> Self {
        Self {
            expected_id,
            invalid_id,
        }
    }

    pub fn expected_id(&self) -> Identifier {
        self.expected_id.clone()
    }

    pub fn invalid_id(&self) -> Identifier {
        self.invalid_id.clone()
    }
}

impl From<InvalidDocumentTransitionIdError> for BasicError {
    fn from(err: InvalidDocumentTransitionIdError) -> Self {
        Self::InvalidDocumentTransitionIdError(err)
    }
}
