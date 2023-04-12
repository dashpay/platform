use crate::identity::KeyType;
use crate::prelude::IdentityPublicKey;
use crate::ProtocolError;
use platform_value::{BinaryData, Value};
use std::collections::HashMap;

pub trait Signer {
    /// the public key bytes are only used to look up the private key
    fn sign(
        &self,
        identity_public_key: &IdentityPublicKey,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError>;
}
