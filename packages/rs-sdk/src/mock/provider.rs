//! Example ContextProvider that uses the Core gRPC API to fetch data from the platform.

use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

use dpp::prelude::{DataContract, Identifier};
use drive_proof_verifier::ContextProvider;

use crate::mock::wallet::core_client::CoreClient;
use crate::platform::Fetch;
use crate::{Error, Sdk};

/// Context provider that uses the Core gRPC API to fetch data from the platform.
///
/// Example [ContextProvider] used by the Sdk for testing purposes.
pub struct GrpcContextProvider<'a> {
    /// Core client
    core: CoreClient,
    /// Sdk to use when fetching data from Platform
    sdk: &'a Sdk,
    /// Data contracts cache.
    ///
    /// Users can insert new data contracts into the cache using [`Cache::put`].
    pub data_contracts: Cache<Identifier, dpp::data_contract::DataContract>,
}

impl<'a> GrpcContextProvider<'a> {
    /// Create new context provider.
    pub fn new(
        sdk: &'a Sdk,

        core_ip: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,
    ) -> Result<Self, Error> {
        let core_client = CoreClient::new(core_ip, core_port, core_user, core_password)?;
        Ok(Self {
            core: core_client,
            sdk,
            data_contracts: Cache::new(NonZeroUsize::new(100).ok_or(Error::Config(
                "data contracts cache capacity must be greater than zero".to_string(),
            ))?),
        })
    }
}

impl<'a> ContextProvider for GrpcContextProvider<'a> {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32], // quorum hash is 32 bytes
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        self.core
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
            .map_err(|e| drive_proof_verifier::Error::InvalidQuorum {
                error: e.to_string(),
            })
    }

    fn get_data_contract(
        &self,
        data_contract_id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, drive_proof_verifier::Error> {
        if let Some(contract) = self.data_contracts.get(data_contract_id) {
            return Ok(Some(contract));
        };

        let handle = match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle,
            // not an error, we rely on the caller to provide a data contract using
            Err(e) => {
                tracing::warn!(
                    error = e.to_string(),
                    "data contract cache miss and no tokio runtime detected, skipping fetch"
                );
                return Ok(None);
            }
        };

        let data_contract = handle
            .block_on(DataContract::fetch(self.sdk, *data_contract_id))
            .map_err(|e| drive_proof_verifier::Error::InvalidDataContract {
                error: e.to_string(),
            })?;

        if let Some(ref dc) = data_contract {
            self.data_contracts.put(*data_contract_id, dc.clone());
        };

        Ok(data_contract.map(Arc::new))
    }
}

/// Thread-safe cache of various objects inside the SDK.
///
/// This is used to cache objects that are expensive to fetch from the platform, like data contracts.
pub struct Cache<K: Hash + Eq, V> {
    // We use a Mutex to allow access to the cache when we don't have mutable &self
    // And we use Arc to allow multiple threads to access the cache without having to clone it
    inner: std::sync::RwLock<lru::LruCache<K, Arc<V>>>,
}

impl<K: Hash + Eq, V> Cache<K, V> {
    /// Create new cache
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            // inner: std::sync::Mutex::new(lru::LruCache::new(capacity)),
            inner: std::sync::RwLock::new(lru::LruCache::new(capacity)),
        }
    }

    /// Get a reference to the value stored under `k`.
    pub fn get(&self, k: &K) -> Option<Arc<V>> {
        let mut guard = self.inner.write().expect("cache lock poisoned");
        guard.get(k).map(Arc::clone)
    }

    /// Insert a new value into the cache.
    pub fn put(&self, k: K, v: V) {
        let mut guard = self.inner.write().expect("cache lock poisoned");
        guard.put(k, Arc::new(v));
    }
}
