use dpp::{BlsModule, ProtocolError, PublicKeyValidationError};

pub struct MockBLS {}

impl BlsModule for MockBLS {
    fn validate_public_key(&self, _pk: &[u8]) -> Result<(), PublicKeyValidationError> {
        panic!("BLS signatures are not implemented");
    }

    fn verify_signature(
        &self,
        _signature: &[u8],
        _data: &[u8],
        _public_key: &[u8],
    ) -> Result<bool, ProtocolError> {
        panic!("BLS signatures are not implemented");
    }

    fn private_key_to_public_key(&self, _private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        panic!("BLS signatures are not implemented");
    }

    fn sign(&self, _data: &[u8], _private_key: &[u8]) -> Result<Vec<u8>, ProtocolError> {
        panic!("BLS signatures are not implemented");
    }
}
