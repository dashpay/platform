use crate::ProtocolError;
use dashcore::{Address, Network};

pub trait IdentityPublicKeyHashMethodsV0 {
    /// Get the original public key hash
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError>;

    /// Get the address
    fn address(&self, network: Network) -> Result<Address, ProtocolError>;

    /// Verifies that the private key bytes match this identity public key
    fn validate_private_key_bytes(
        &self,
        private_key_bytes: &[u8; 32],
        network: Network,
    ) -> Result<bool, ProtocolError>;
}
