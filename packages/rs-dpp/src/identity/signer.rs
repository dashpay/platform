use crate::prelude::IdentityPublicKey;
use crate::ProtocolError;
use platform_value::BinaryData;

pub trait Signer: Sync {
    /// the public key bytes are only used to look up the private key
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError>;
}
