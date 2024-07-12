//! Example ContextProvider that uses the Core gRPC API to fetch data from the platform.

use crate::core_client::CoreClient;
use crate::platform::Fetch;
use crate::{Error, Sdk};
use arc_swap::ArcSwapAny;
use dpp::prelude::{DataContract, Identifier};
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;
use pollster::FutureExt;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Context provider that uses the Core gRPC API to fetch data from the platform.
///
/// Example [ContextProvider] used by the Sdk for testing purposes.
pub struct GrpcContextProvider {
    /// Core client
    core: CoreClient,
    /// [Sdk] to use when fetching data from Platform
    ///
    /// Note that if the `sdk` is `None`, the context provider will not be able to fetch data itself and will rely on
    /// values set by the user in the caches: `data_contracts_cache`, `quorum_public_keys_cache`.
    ///
    /// We use [Arc] as we have circular dependencies between Sdk and ContextProvider.
    sdk: ArcSwapAny<Arc<Option<Sdk>>>,

    /// Data contracts cache.
    ///
    /// Users can insert new data contracts into the cache using [`Cache::put`].
    pub data_contracts_cache: Cache<Identifier, dpp::data_contract::DataContract>,

    /// Quorum public keys cache.
    ///
    /// Key is a tuple of quorum hash and quorum type. Value is a quorum public key.
    ///
    /// Users can insert new quorum public keys into the cache using [`Cache::put`].
    pub quorum_public_keys_cache: Cache<([u8; 32], u32), [u8; 48]>,

    /// Directory where to store dumped data.
    ///
    /// This is used to store data that is fetched from the platform and can be used for testing purposes.
    #[cfg(feature = "mocks")]
    pub dump_dir: Option<std::path::PathBuf>,
}

impl GrpcContextProvider {
    /// Create new context provider.
    ///
    /// Note that if the `sdk` is `None`, the context provider will not be able to fetch data itself and will rely on
    /// values set by the user in the caches: `data_contracts_cache`, `quorum_public_keys_cache`.
    ///
    /// Sdk can be set later with [`GrpcContextProvider::set_sdk`].
    pub fn new(
        sdk: Option<Sdk>,
        core_ip: &str,
        core_port: u16,
        core_user: &str,
        core_password: &str,

        data_contracts_cache_size: NonZeroUsize,
        quorum_public_keys_cache_size: NonZeroUsize,
    ) -> Result<Self, Error> {
        let core_client = CoreClient::new(core_ip, core_port, core_user, core_password)?;
        Ok(Self {
            core: core_client,
            sdk: ArcSwapAny::new(Arc::new(sdk)),
            data_contracts_cache: Cache::new(data_contracts_cache_size),
            quorum_public_keys_cache: Cache::new(quorum_public_keys_cache_size),
            #[cfg(feature = "mocks")]
            dump_dir: None,
        })
    }

    /// Set the Sdk to use when fetching data from Platform.
    /// This is useful when the Sdk is created after the ContextProvider.
    ///
    /// Note that if the `sdk` is `None`, the context provider will not be able to fetch data itself and will rely on
    /// values set by the user in the caches: `data_contracts_cache`, `quorum_public_keys_cache`.
    pub fn set_sdk(&self, sdk: Option<Sdk>) {
        self.sdk.store(Arc::new(sdk));
    }
    /// Set the directory where to store dumped data.
    ///
    /// When set, the context provider will store data fetched from the platform into this directory.
    #[cfg(feature = "mocks")]
    pub fn set_dump_dir(&mut self, dump_dir: Option<std::path::PathBuf>) {
        self.dump_dir = dump_dir;
    }

    /// Save quorum public key to disk.
    ///
    /// Files are named: `quorum_pubkey-<int_quorum_type>-<hex_quorum_hash>.json`
    ///
    /// Note that this will overwrite files with the same quorum type and quorum hash.
    ///
    /// Any errors are logged on `warn` level and ignored.
    #[cfg(feature = "mocks")]
    fn dump_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
        public_key: &[u8],
    ) {
        use hex::ToHex;

        let path = match &self.dump_dir {
            Some(p) => p,
            None => return,
        };

        let encoded = hex::encode(public_key);

        let file = path.join(format!(
            "quorum_pubkey-{}-{}.json",
            quorum_type,
            quorum_hash.encode_hex::<String>()
        ));

        if let Err(e) = std::fs::write(file, encoded) {
            tracing::warn!("Unable to write dump file {:?}: {}", path, e);
        }
    }

    /// Save data contract to disk.
    ///
    /// Files are named: `quorum_pubkey-<int_quorum_type>-<hex_quorum_hash>.json`
    ///
    /// Note that this will overwrite files with the same quorum type and quorum hash.
    ///
    /// Any errors are logged on `warn` level and ignored.
    #[cfg(feature = "mocks")]
    fn dump_data_contract(&self, data_contract: &DataContract) {
        use dpp::data_contract::accessors::v0::DataContractV0Getters;
        use hex::ToHex;

        let path = match &self.dump_dir {
            Some(p) => p,
            None => return,
        };
        let id = data_contract.id();

        let file = path.join(format!("data_contract-{}.json", id.encode_hex::<String>()));

        let encoded = serde_json::to_vec(data_contract).expect("serialize data contract");
        if let Err(e) = std::fs::write(file, encoded) {
            tracing::warn!("Unable to write dump file {:?}: {}", path, e);
        }
    }
}

impl ContextProvider for GrpcContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32], // quorum hash is 32 bytes
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        if let Some(key) = self
            .quorum_public_keys_cache
            .get(&(quorum_hash, quorum_type))
        {
            return Ok(*key);
        };

        let key = self.core.get_quorum_public_key(quorum_type, quorum_hash)?;

        self.quorum_public_keys_cache
            .put((quorum_hash, quorum_type), key);

        #[cfg(feature = "mocks")]
        self.dump_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height, &key);

        Ok(key)
    }

    fn get_data_contract(
        &self,
        data_contract_id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if let Some(contract) = self.data_contracts_cache.get(data_contract_id) {
            return Ok(Some(contract));
        };
        let sdk_guard = self.sdk.load();

        let sdk = match sdk_guard.as_ref() {
            Some(sdk) => sdk,
            None => {
                tracing::warn!("data contract cache miss and no sdk provided, skipping fetch");
                return Ok(None);
            }
        };

        let contract_id = *data_contract_id;

        let sdk_cloned = sdk.clone();

        let data_contract: Option<DataContract> = DataContract::fetch(&sdk_cloned, contract_id)
            .block_on()
            .map_err(|e| ContextProviderError::DataContractFailure(e.to_string()))?;

        if let Some(ref dc) = data_contract {
            self.data_contracts_cache.put(*data_contract_id, dc.clone());
        };

        #[cfg(feature = "mocks")]
        if let Some(ref dc) = data_contract {
            self.dump_data_contract(dc);
        }

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
