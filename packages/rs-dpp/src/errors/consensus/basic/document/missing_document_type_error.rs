use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use bincode::{Decode, Encode};

#[derive(Error, Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, Encode, Decode)]
#[error("$type is not present")]
pub struct MissingDocumentTypeError;

/*

DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

*/

impl MissingDocumentTypeError {
    pub fn new() -> Self {
        Self
    }
}

impl From<MissingDocumentTypeError> for ConsensusError {
    fn from(err: MissingDocumentTypeError) -> Self {
        Self::BasicError(BasicError::MissingDocumentTypeError(err))
    }
}
