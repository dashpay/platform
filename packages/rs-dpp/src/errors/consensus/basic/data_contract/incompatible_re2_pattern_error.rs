use crate::errors::consensus::basic::BasicError;
use crate::errors::ProtocolError;
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

use bincode::{Decode, Encode};

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error("Pattern '{pattern}' at '{path}' is not not compatible with Re2: {message}")]
#[platform_serialize(unversioned)]
#[ferment_macro::export]
pub struct IncompatibleRe2PatternError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    pub pattern: String,
    pub path: String,
    pub message: String,
}

impl IncompatibleRe2PatternError {
    pub fn new(pattern: String, path: String, message: String) -> Self {
        Self {
            pattern,
            path,
            message,
        }
    }

    pub fn pattern(&self) -> String {
        self.pattern.clone()
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn message(&self) -> String {
        self.message.clone()
    }
}

impl From<IncompatibleRe2PatternError> for ConsensusError {
    fn from(err: IncompatibleRe2PatternError) -> Self {
        Self::BasicError(BasicError::IncompatibleRe2PatternError(err))
    }
}
