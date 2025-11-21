use crate::prelude::IdentityPublicKey;
use crate::ProtocolError;
use platform_value::BinaryData;
use std::fmt::Debug;

pub trait Signer<K>: Sync + Debug {
    /// the public key bytes are only used to look up the private key
    fn sign(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<BinaryData, ProtocolError>;

    /// do we have this identity public key in the signer?
    fn can_sign_with(&self, key: &K) -> bool;
}

pub trait IdentitySigner = Signer<IdentityPublicKey>;