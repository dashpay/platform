use thiserror::Error;

use crate::consensus::state::state_error::StateError;

use crate::consensus::fee::fee_error::FeeError;
use crate::consensus::signature::signature_error::SignatureError;
#[cfg(test)]
use crate::errors::consensus::basic::TestConsensusError;
use crate::errors::consensus::basic::{BasicError, JsonSchemaError};
use platform_value::Error as ValueError;

#[derive(Error, Debug)]
//#[cfg_attr(test, derive(Clone))]
pub enum ConsensusError {
    // TODO: Why do we use Box?
    #[error(transparent)]
    StateError(Box<StateError>),

    #[error(transparent)]
    BasicError(Box<BasicError>),

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
            Self::BasicError(e) => match **e {
                BasicError::JsonSchemaError(err) => Some(&err),
                _ => None,
            },
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
