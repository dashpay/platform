use bincode::Options;
use crate::ProtocolError;
use crate::state_transition::StateTransition;

impl StateTransition {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .serialize(self)
            .map_err(|_| {
                ProtocolError::EncodingError(String::from(
                    "unable to serialize identity public key",
                ))
            })
    }

    pub fn serialized_size(&self) -> usize {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .serialized_size(self)
            .unwrap() as usize // this should not be able to error
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        bincode::DefaultOptions::default()
            .with_varint_encoding()
            .reject_trailing_bytes()
            .with_big_endian()
            .deserialize(bytes)
            .map_err(|e| ProtocolError::EncodingError(format!("unable to deserialize key {}", e)))
    }

    pub fn deserialize_many(raw_state_transitions: &Vec<Vec<u8>>) -> Result<Vec<Self>, ProtocolError> {
        raw_state_transitions.iter().map(|raw_state_transition| {
            Self::deserialize(raw_state_transition)
        }).collect()
    }
}