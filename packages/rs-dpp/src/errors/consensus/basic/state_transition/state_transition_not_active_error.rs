use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::errors::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use platform_serialization_derive::{
    PlatformDeserializeTrusted, PlatformDeserializeUntrusted, PlatformSerialize,
};
use platform_version::version::ProtocolVersion;
use thiserror::Error;

#[derive(
    Error,
    Debug,
    Clone,
    PartialEq,
    Eq,
    Encode,
    Decode,
    PlatformSerialize,
    PlatformDeserializeTrusted,
    PlatformDeserializeUntrusted,
    DecodeUntrusted,
)]
#[error("{}", self.describe())]
#[platform_serialize(unversioned)]
pub struct StateTransitionNotActiveError {
    /*

    DO NOT CHANGE ORDER OF FIELDS WITHOUT INTRODUCING OF NEW VERSION

    */
    state_transition_type: String,
    current_protocol_version: ProtocolVersion,
    required_protocol_version: ProtocolVersion,
}

impl StateTransitionNotActiveError {
    pub fn new(
        state_transition_type: impl Into<String>,
        current_protocol_version: ProtocolVersion,
        required_protocol_version: ProtocolVersion,
    ) -> Self {
        Self {
            state_transition_type: state_transition_type.into(),
            current_protocol_version,
            required_protocol_version,
        }
    }

    pub fn state_transition_type(&self) -> &str {
        &self.state_transition_type
    }

    pub fn current_protocol_version(&self) -> ProtocolVersion {
        self.current_protocol_version
    }

    /// The boundary of the transition's active range that `current_protocol_version` missed, which
    /// is the range's start when the version is below it and the range's end when it is above.
    /// Read it with `current_protocol_version`: alone it does not say which side was missed, and a
    /// version above the range is not reached by moving to the one reported.
    pub fn required_protocol_version(&self) -> ProtocolVersion {
        self.required_protocol_version
    }

    /// Both sides of the range read differently, so each is spelled out rather than sharing a
    /// wording that would be accurate for both and actionable for neither.
    fn describe(&self) -> String {
        if self.current_protocol_version > self.required_protocol_version {
            format!(
                "State transition type {} is no longer accepted at protocol version {}: protocol \
                 version {} was the last that accepted it, and a newer transition version carries \
                 it from here",
                self.state_transition_type,
                self.current_protocol_version,
                self.required_protocol_version
            )
        } else {
            format!(
                "State transition type {} is not active at protocol version {}: it becomes active \
                 at protocol version {}",
                self.state_transition_type,
                self.current_protocol_version,
                self.required_protocol_version
            )
        }
    }
}

impl From<StateTransitionNotActiveError> for ConsensusError {
    fn from(err: StateTransitionNotActiveError) -> Self {
        Self::BasicError(BasicError::StateTransitionNotActiveError(err))
    }
}
