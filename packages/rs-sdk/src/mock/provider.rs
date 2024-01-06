//! Example ContextProvider that uses the Core gRPC API to fetch data from the platform.

use crate::common::cache::Cache;
use crate::mock::wallet::core_grpc_wallet::CoreClient;
use crate::platform::Fetch;
use crate::{Error, Sdk};
use dpp::prelude::{DataContract, Identifier};
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::ContextProvider;
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
    sdk: Option<Arc<Sdk>>,

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
}

impl GrpcContextProvider {
    /// Create new context provider.
    ///
    /// Note that if the `sdk` is `None`, the context provider will not be able to fetch data itself and will rely on
    /// values set by the user in the caches: `data_contracts_cache`, `quorum_public_keys_cache`.
    ///
    /// Sdk can be set later with [`GrpcContextProvider::set_sdk`].
    pub fn new(
        sdk: Option<Arc<Sdk>>,
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
            sdk,
            data_contracts_cache: Cache::new(data_contracts_cache_size),
            quorum_public_keys_cache: Cache::new(quorum_public_keys_cache_size),
        })
    }

    /// Set the Sdk to use when fetching data from Platform.
    /// This is useful when the Sdk is created after the ContextProvider.
    ///
    /// Note that if the `sdk` is `None`, the context provider will not be able to fetch data itself and will rely on
    /// values set by the user in the caches: `data_contracts_cache`, `quorum_public_keys_cache`.
    pub fn set_sdk(&mut self, sdk: Option<Arc<Sdk>>) {
        self.sdk = sdk;
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

        let key =
            self.core
                .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)?;

        self.quorum_public_keys_cache
            .put((quorum_hash, quorum_type), key);

        Ok(key)
    }

    fn get_data_contract(
        &self,
        data_contract_id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        if let Some(contract) = self.data_contracts_cache.get(data_contract_id) {
            return Ok(Some(contract));
        };

        let sdk = match &self.sdk {
            Some(sdk) => sdk,
            None => {
                tracing::warn!("data contract cache miss and no sdk provided, skipping fetch");
                return Ok(None);
            }
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
            .block_on(DataContract::fetch(sdk, *data_contract_id))
            .map_err(|e| ContextProviderError::InvalidDataContract(e.to_string()))?;

        if let Some(ref dc) = data_contract {
            self.data_contracts_cache.put(*data_contract_id, dc.clone());
        };

        Ok(data_contract.map(Arc::new))
    }
}
