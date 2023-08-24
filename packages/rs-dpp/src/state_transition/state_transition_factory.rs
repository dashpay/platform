use crate::consensus::basic::decode::SerializedObjectParsingError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization::PlatformDeserializable;
use crate::state_transition::StateTransition;
use crate::ProtocolError;

#[derive(Clone)]
pub struct StateTransitionFactory;

impl StateTransitionFactory {
    pub fn create_from_buffer(&self, buffer: &[u8]) -> Result<StateTransition, ProtocolError> {
        StateTransition::deserialize(buffer).map_err(|e| {
            ConsensusError::BasicError(BasicError::SerializedObjectParsingError(
                SerializedObjectParsingError::new(format!("Decode protocol entity: {:#?}", e)),
            ))
            .into()
        })
    }
}
