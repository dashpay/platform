use crate::identity::identity_public_key::methods::hash::IdentityPublicKeyHashMethodsV0;
use crate::identity::identity_public_key::v0::methods::{
    public_key_hash_for_key_data, validate_private_key_bytes_for_key_data,
};
use crate::identity::identity_public_key::v1::IdentityPublicKeyV1;
use crate::ProtocolError;
use dashcore::Network;

impl IdentityPublicKeyHashMethodsV0 for IdentityPublicKeyV1 {
    /// Get the original public key hash
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError> {
        public_key_hash_for_key_data(self.key_type, &self.data)
    }

    fn validate_private_key_bytes(
        &self,
        private_key_bytes: &[u8; 32],
        network: Network,
    ) -> Result<bool, ProtocolError> {
        validate_private_key_bytes_for_key_data(
            self.key_type,
            &self.data,
            private_key_bytes,
            network,
        )
    }
}
