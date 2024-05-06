mod v0;

use crate::identity::identity_public_key::IdentityPublicKey;
use crate::errors::ProtocolError;
pub use v0::*;

impl IdentityPublicKeyHashMethodsV0 for IdentityPublicKey {
    fn hash(&self) -> Result<[u8; 20], ProtocolError> {
        match self {
            IdentityPublicKey::V0(v0) => v0.hash(),
        }
    }
}
