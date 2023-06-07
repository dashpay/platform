use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use bincode::{Decode, Encode};
use platform_value::Error as PlatformValueError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
#[error("{value_error}")]
pub struct ValueError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    value_error: String,
}

impl ValueError {
    pub fn new(value_error: PlatformValueError) -> Self {
        Self {
            value_error: value_error.to_string(),
        }
    }

    pub fn value_error(&self) -> &str {
        &self.value_error
    }
}
impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::BasicError(BasicError::ValueError(err))
    }
}
