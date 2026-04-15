use crate::address_funds::AddressWitness;
use crate::ProtocolError;
use platform_value::BinaryData;
use std::fmt::Debug;
use std::sync::Arc;

pub trait Signer<K>: Sync + Debug {
    /// the public key bytes are only used to look up the private key
    fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError>;

    /// the public key bytes are only used to look up the private key
    fn sign_create_witness(&self, key: &K, data: &[u8]) -> Result<AddressWitness, ProtocolError>;

    /// do we have this identity public key in the signer?
    fn can_sign_with(&self, key: &K) -> bool;
}

impl<K, S> Signer<K> for Arc<S>
where
    S: Signer<K> + ?Sized + Send,
{
    fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        (**self).sign(key, data)
    }

    fn sign_create_witness(&self, key: &K, data: &[u8]) -> Result<AddressWitness, ProtocolError> {
        (**self).sign_create_witness(key, data)
    }

    fn can_sign_with(&self, key: &K) -> bool {
        (**self).can_sign_with(key)
    }
}

/// Async version of the [Signer] trait for use in async contexts.
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
pub trait SignerAsync<K>: Send + Debug {
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError>;
    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError>;
    fn can_sign_with(&self, key: &K) -> bool;
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<K: Send + Sync, S> SignerAsync<K> for Arc<S>
where
    S: SignerAsync<K> + ?Sized + Sync,
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
