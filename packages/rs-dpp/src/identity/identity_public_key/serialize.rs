use crate::identity::IdentityPublicKey;
use crate::ProtocolError;
use bincode::Options;

impl IdentityPublicKey {
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
            .map_err(|_| {
                ProtocolError::EncodingError(String::from("unable to deserialize element"))
            })
    }
}
