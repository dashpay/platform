#[cfg(all(not(target_arch = "wasm32"), feature = "bls-signatures"))]
pub mod native_bls;

use crate::{ProtocolError, PublicKeyValidationError};

pub trait BlsModule {
    fn validate_public_key(&self, pk: &[u8]) -> Result<(), PublicKeyValidationError>;
    fn verify_signature(
        &self,
        signature: &[u8],
        data: &[u8],
        public_key: &[u8],
    ) -> Result<bool, ProtocolError>;
    fn private_key_to_public_key(&self, private_key: &[u8]) -> Result<Vec<u8>, ProtocolError>;
    fn sign(&self, data: &[u8], private_key: &[u8]) -> Result<Vec<u8>, ProtocolError>;
}
