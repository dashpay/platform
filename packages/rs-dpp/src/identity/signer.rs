use crate::address_funds::AddressWitness;
use crate::ProtocolError;
use async_trait::async_trait;
use platform_value::BinaryData;
use std::fmt::Debug;
use std::sync::Arc;

// `#[async_trait]` is required: `Signer` is used dyn — stored as
// `Arc<dyn Signer<_>>` in `VTableSigner` (rs-sdk-ffi/src/signer.rs).
#[async_trait]
pub trait Signer<K>: Send + Sync + Debug {
    /// the public key bytes are only used to look up the private key
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError>;

    /// the public key bytes are only used to look up the private key
    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError>;

    /// do we have this identity public key in the signer?
    fn can_sign_with(&self, key: &K) -> bool;
}

#[async_trait]
impl<K, S> Signer<K> for Arc<S>
where
    K: Send + Sync,
    S: Signer<K> + ?Sized + Send + Sync,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        (**self).sign(key, data).await
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        (**self).sign_create_witness(key, data).await
    }

    fn can_sign_with(&self, key: &K) -> bool {
        (**self).can_sign_with(key)
    }
}
