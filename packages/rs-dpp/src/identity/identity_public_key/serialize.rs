use crate::identity::IdentityPublicKey;
use crate::ProtocolError;
use bincode::config;

impl IdentityPublicKey {
    pub fn serialize(&self) -> Result<Vec<u8>, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<2000>();
        bincode::encode_to_vec(self, config).map_err(|_| {
            ProtocolError::EncodingError(String::from("unable to serialize identity public key"))
        })
    }

    pub fn serialized_size(&self) -> Result<usize, ProtocolError> {
        self.serialize().map(|a| a.len())
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let config = config::standard().with_big_endian().with_limit::<2000>();
        bincode::decode_from_slice(bytes, config)
            .map_err(|e| ProtocolError::EncodingError(format!("unable to deserialize key {}", e)))
            .map(|(a, _)| a)
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::IdentityPublicKey;

    #[test]
    fn test_identity_key_serialization_deserialization() {
        let key = IdentityPublicKey::random_key(1, Some(500));
        let serialized = key.serialize().expect("expected to serialize key");
        let unserialized = IdentityPublicKey::deserialize(serialized.as_slice())
            .expect("expected to deserialize key");
        assert_eq!(key, unserialized)
    }
}
