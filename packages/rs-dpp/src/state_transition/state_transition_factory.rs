use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization::PlatformDeserializableUntrusted;
use crate::state_transition::StateTransition;
use crate::ProtocolError;

#[derive(Clone)]
pub struct StateTransitionFactory;

impl StateTransitionFactory {
    pub fn create_from_buffer(&self, buffer: &[u8]) -> Result<StateTransition, ProtocolError> {
        let parsing_error = |e: ProtocolError| -> ProtocolError {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
            .into()
        };
        let state_transition =
            StateTransition::deserialize_from_bytes_untrusted(buffer).map_err(parsing_error)?;
        state_transition
            .ensure_admitted_by_some_version()
            .map_err(parsing_error)?;
        Ok(state_transition)
    }
}
