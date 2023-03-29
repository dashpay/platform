use thiserror::Error;
use crate::consensus::ConsensusError;
use platform_value::{Value};

#[derive(Debug, Error, Clone, Eq, PartialEq)]
#[error("Invalid Identity: {errors:?}")]
pub struct InvalidIdentityError {
    errors: Vec<ConsensusError>,
    raw_identity: Value,
}

impl InvalidIdentityError {
    pub fn new(errors: Vec<ConsensusError>, raw_identity: Value) -> Self {
        Self {
            errors,
            raw_identity,
        }
    }

    pub fn errors(&self) -> Vec<ConsensusError> {
        self.errors.clone()
    }

    pub fn raw_identity(&self) -> Value {
        self.raw_identity.clone()
    }
}
