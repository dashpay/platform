use thiserror::Error;

use crate::consensus::state::state_error::StateError;

use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::signature_error::SignatureError;

#[cfg(test)]
use crate::consensus::test_consensus_error::TestConsensusError;

use crate::errors::consensus::basic::{BasicError, JsonSchemaError};
use platform_value::Error as ValueError;

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error(transparent)]
    StateError(StateError),

    #[error(transparent)]
    BasicError(BasicError),

    #[error(transparent)]
    SignatureError(SignatureError),

    #[error(transparent)]
    FeeError(FeeError),

    #[error(transparent)]
    ValueError(ValueError),

    #[cfg(test)]
    #[cfg_attr(test, error(transparent))]
    TestConsensusError(TestConsensusError),
}

impl ConsensusError {
    // TODO: Not sure it should be here. Looks more like a test helper
    pub fn json_schema_error(&self) -> Option<&JsonSchemaError> {
        match self {
            Self::BasicError(BasicError::JsonSchemaError(err)) => Some(err),
            _ => None,
        }
    }

    pub fn value_error(&self) -> Option<&ValueError> {
        match self {
            ConsensusError::ValueError(err) => Some(err),
            _ => None,
        }
    }
}

impl From<FeeError> for ConsensusError {
    fn from(err: FeeError) -> Self {
        Self::FeeError(err)
    }
}

impl From<ValueError> for ConsensusError {
    fn from(err: ValueError) -> Self {
        Self::ValueError(err)
    }
}

#[cfg(test)]
impl From<TestConsensusError> for ConsensusError {
    fn from(error: TestConsensusError) -> Self {
        Self::TestConsensusError(error)
    }
}
