//! Adapter that bridges between [Signer] (sync) and [SignerAsync] (async).

use dpp::address_funds::AddressWitness;
use dpp::identity::signer::{Signer, SignerAsync};
use dpp::platform_value::BinaryData;
use dpp::ProtocolError;
use std::fmt::Debug;
use std::sync::Arc;

use crate::sync::block_on;

/// Wraps a synchronous [Signer] and exposes it as [SignerAsync].
///
/// Useful when async callers need to invoke a sync signer without blocking
/// the executor.
#[derive(Debug, Clone)]
pub struct SyncSignerAdapter<S>(pub Arc<S>);

impl<S> SyncSignerAdapter<S> {
    pub fn new(signer: S) -> Self {
        Self(Arc::new(signer))
    }

    pub fn from_arc(signer: Arc<S>) -> Self {
        Self(signer)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
impl<K, S> SignerAsync<K> for SyncSignerAdapter<S>
where
    K: Send + Sync + Debug + 'static,
    S: Signer<K> + Send + 'static,
{
    async fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        self.0.sign(key, data)
    }

    async fn sign_create_witness(
        &self,
        key: &K,
        data: &[u8],
    ) -> Result<AddressWitness, ProtocolError> {
        self.0.sign_create_witness(key, data)
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}

/// Wraps an async [SignerAsync] and exposes it as a sync [Signer].
///
/// Uses [block_on] internally to drive the future to completion.
/// Should only be used when a sync interface is required but the
/// underlying implementation is async.
///
/// Requires `K: Clone` so the key can be moved into the spawned future
/// without unsafe pointer tricks.
#[derive(Debug, Clone)]
pub struct AsyncSignerAdapter<S>(pub Arc<S>);

impl<S> AsyncSignerAdapter<S> {
    pub fn new(signer: S) -> Self {
        Self(Arc::new(signer))
    }

    pub fn from_arc(signer: Arc<S>) -> Self {
        Self(signer)
    }
}

impl<K, S> Signer<K> for AsyncSignerAdapter<S>
where
    K: Send + Sync + Clone + Debug + 'static,
    S: SignerAsync<K> + Sync + 'static,
{
    fn sign(&self, key: &K, data: &[u8]) -> Result<BinaryData, ProtocolError> {
        let signer = self.0.clone();
        let key = key.clone();
        let data = data.to_vec();
        block_on(async move { signer.sign(&key, &data).await })
            .map_err(|e| ProtocolError::Generic(format!("async-sync bridge error: {e}")))?
    }

    fn sign_create_witness(&self, key: &K, data: &[u8]) -> Result<AddressWitness, ProtocolError> {
        let signer = self.0.clone();
        let key = key.clone();
        let data = data.to_vec();
        block_on(async move { signer.sign_create_witness(&key, &data).await })
            .map_err(|e| ProtocolError::Generic(format!("async-sync bridge error: {e}")))?
    }

    fn can_sign_with(&self, key: &K) -> bool {
        self.0.can_sign_with(key)
    }
}
