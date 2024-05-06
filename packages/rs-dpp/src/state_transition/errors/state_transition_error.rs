use platform_value::Value;
use thiserror::Error;

use crate::errors::consensus::ConsensusError;

#[derive(Error, Debug)]
#[ferment_macro::export]
pub enum StateTransitionError {
    #[error("Invalid State Transition: {errors:?}")]
    InvalidStateTransitionError {
        errors: Vec<ConsensusError>,
        raw_state_transition: Value,
    },
}
