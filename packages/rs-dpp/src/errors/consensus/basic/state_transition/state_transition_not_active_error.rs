use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, Encode};
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
use thiserror::Error;
use platform_version::version::ProtocolVersion;
use crate::state_transition::StateTransitionType;

#[derive(
    Error, Debug, Clone, PartialEq, Eq, Encode, Decode, PlatformSerialize, PlatformDeserialize,
)]
#[error(
    "State transition type {state_transition_type} is not yet active. Current protocol version is {current_protocol_version}, required protocol version is {required_protocol_version}"
)]
#[platform_serialize(unversioned)]
pub struct StateTransitionNotActiveError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    state_transition_type: StateTransitionType,
    current_protocol_version: ProtocolVersion,
    required_protocol_version: ProtocolVersion,
}

impl StateTransitionNotActiveError {
    pub fn new(
        state_transition_type: StateTransitionType,
        current_protocol_version: ProtocolVersion,
        required_protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            state_transition_type,
            current_protocol_version,
            required_protocol_version,
        }
    }

    pub fn state_transition_type(&self) -> StateTransitionType {
        self.state_transition_type
    }

    pub fn current_protocol_version(&self) -> ProtocolVersion {
        self.current_protocol_version
    }

    pub fn required_protocol_version(&self) -> ProtocolVersion {
        self.required_protocol_version
    }
}

impl From<StateTransitionNotActiveError> for ConsensusError {
    fn from(err: StateTransitionNotActiveError) -> Self {
        Self::BasicError(BasicError::StateTransitionNotActiveError(err))
    }
}
