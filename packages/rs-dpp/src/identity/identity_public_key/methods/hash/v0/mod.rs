use crate::ProtocolError;

pub trait IdentityPublicKeyHashMethodsV0 {
    /// Get the original public key hash
    fn public_key_hash(&self) -> Result<[u8; 20], ProtocolError>;
}
