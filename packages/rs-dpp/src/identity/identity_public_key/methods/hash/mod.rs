mod v0;

use crate::identity::IdentityPublicKey;
use crate::ProtocolError;
pub use v0::*;

impl IdentityPublicKeyHashMethodsV0 for IdentityPublicKey {
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError> {
        match self {
            IdentityPublicKey::V0(v0) => v0.public_key_hash(),
        }
    }
}
