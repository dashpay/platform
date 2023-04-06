use platform_value::Value;
use thiserror::Error;

use crate::consensus::ConsensusError;
use crate::state_transition::fee::errors::FeeError;

#[derive(Error, Debug)]
pub enum StateTransitionError {
    #[error("Invalid State Transition: {errors:?}")]
    InvalidStateTransitionError {
        errors: Vec<ConsensusError>,
        raw_state_transition: Value,
    },
    #[error(transparent)]
    FeeError(FeeError),
}
