use crate::prelude::Identity;
use crate::ProtocolError;
use bincode::config;

impl Identity {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<15000>();
        bincode::encode_to_vec(self, config)
            .map_err(|_| ProtocolError::EncodingError(String::from("unable to serialize identity")))
    }

    pub fn serialized_size(&self) -> Result<usize, ProtocolError> {
        self.serialize().map(|a| a.len())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<15000>();
        bincode::decode_from_slice(bytes, config)
            .map_err(|e| {
                ProtocolError::EncodingError(format!("unable to deserialize identity {}", e))
            })
            .map(|(a, _)| a)
    }
}
