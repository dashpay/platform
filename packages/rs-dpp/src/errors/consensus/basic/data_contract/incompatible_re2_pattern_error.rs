use thiserror::Error;

use crate::consensus::ConsensusError;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("Pattern '{pattern}' at '{path}' is not not compatible with Re2: {message}")]
pub struct IncompatibleRe2PatternError {
    pattern: String,
    path: String,
    message: String,
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
        Self::IncompatibleRe2PatternError(err)
    }
}
