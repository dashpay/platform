use crate::consensus::ConsensusError;
use platform_value::Value;
use platform_version::version::ProtocolVersion;
use std::ops::RangeInclusive;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateTransitionError {
    #[error("Invalid State Transition: {errors:?}")]
    InvalidStateTransitionError {
        errors: Vec<ConsensusError>,
        raw_state_transition: Value,
    },

    #[error("The state transition of type '{state_transition_type}' is not active in the current protocol version {current_protocol_version}. For some state transitions this could be because of feature it contains. Active version range: {active_version_range:?}")]
    StateTransitionIsNotActiveError {
        state_transition_type: String,
        active_version_range: RangeInclusive<ProtocolVersion>,
        current_protocol_version: ProtocolVersion,
    },
}
