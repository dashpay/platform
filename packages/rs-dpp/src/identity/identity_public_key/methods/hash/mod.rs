mod v0;

use crate::identity::IdentityPublicKey;
use crate::ProtocolError;
use dashcore::{Address, Network};
pub use v0::*;

impl IdentityPublicKeyHashMethodsV0 for IdentityPublicKey {
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError> {
        match self {
            IdentityPublicKey::V0(v0) => v0.public_key_hash(),
        }
    }

    fn address(&self, network: Network) -> Result<Address, ProtocolError> {
        match self {
            IdentityPublicKey::V0(v0) => v0.address(network),
        }
    }

    fn validate_private_key_bytes(
        &self,
        private_key_bytes: &[u8; 32],
        network: Network,
    ) -> Result<bool, ProtocolError> {
        match self {
            IdentityPublicKey::V0(v0) => v0.validate_private_key_bytes(private_key_bytes, network),
        }
    }
}
