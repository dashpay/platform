use dpp::BlsModule;

/// BLS implementation used in Drive
pub(crate) struct DriveBls {}

impl BlsModule for DriveBls {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), dpp::PublicKeyValidationError> {
        todo!()
    }

    fn verify_signature(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, dpp::ProtocolError> {
        todo!()
    }

    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, dpp::ProtocolError> {
        todo!()
    }

    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, dpp::ProtocolError> {
        todo!()
    }
}
