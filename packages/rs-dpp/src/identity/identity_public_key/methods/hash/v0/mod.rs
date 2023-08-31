use crate::ProtocolError;

pub trait IdentityPublicKeyHashMethodsV0 {
    /// Get the original public key hash
    fn hash(&self) -> Result<[u8; 20], ProtocolError>;
}
