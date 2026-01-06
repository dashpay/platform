use std::num::NonZeroUsize;
use std::sync::Arc;

use dash_sdk::dpp::dashcore::Network;
use dash_sdk::platform::ContextProvider;
use dash_sdk::{
    dpp::{data_contract::TokenConfiguration, prelude::CoreBlockHeight, version::PlatformVersion},
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use wasm_bindgen::prelude::wasm_bindgen;

/// A caching context provider for non-trusted WASM mode.
/// Caches data contracts but does NOT verify proofs (no quorum access).
/// If cache initialization fails, operates in non-caching mode gracefully.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmContext {
    inner: Option<Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>>,
}

/// A wrapper for TrustedHttpContextProvider that works in WASM
#[derive(Clone)]
pub struct WasmTrustedContext {
    inner: std::sync::Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>,
}

impl WasmContext {
    /// Create a new WasmContext for mainnet
    pub fn new_mainnet() -> Self {
        Self::new(Network::Dash)
    }

    /// Create a new WasmContext for testnet
    pub fn new_testnet() -> Self {
        Self::new(Network::Testnet)
    }

    /// Create a new WasmContext for local/regtest
    pub fn new_local() -> Self {
        Self::new(Network::Regtest)
    }

    /// Create a new WasmContext for the specified network.
    /// If initialization fails, returns a context that operates without caching.
    pub fn new(network: Network) -> Self {
        match rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            network,
            None,                            // No fallback provider
            NonZeroUsize::new(100).unwrap(), // Quorum cache size (unused but required)
        ) {
            Ok(provider) => Self {
                inner: Some(Arc::new(provider)),
            },
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize contract cache for {:?}: {}. Operating without cache.",
                    network,
                    e
                );
                Self { inner: None }
            }
        }
    }

    /// Add a data contract to the cache
    pub fn cache_contract(&self, contract: DataContract) {
        if let Some(ref inner) = self.inner {
            inner.add_known_contract(contract);
        }
    }

    /// Invalidate a cached contract (for reactive refresh when schema mismatch detected)
    pub fn invalidate_contract(&self, id: &Identifier) {
        if let Some(ref inner) = self.inner {
            inner.remove_known_contract(id);
        }
    }

    /// Add a token configuration to the cache
    pub fn cache_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        if let Some(ref inner) = self.inner {
            inner.add_known_token_configuration(token_id, config);
        }
    }

    /// Check if caching is enabled
    pub fn has_cache(&self) -> bool {
        self.inner.is_some()
    }
}

impl ContextProvider for WasmContext {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Non-trusted mode does not verify proofs
        Err(ContextProviderError::Generic(
            "Non-trusted mode does not support proof verification. Use trusted SDK builders (new_mainnet_trusted or new_testnet_trusted) for proof verification.".to_string()
        ))
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // Delegate to inner provider's cache if available
        if let Some(ref inner) = self.inner {
            inner.get_data_contract(id, platform_version)
        } else {
            // No cache available, return None (will be fetched from network)
            Ok(None)
        }
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        if let Some(ref inner) = self.inner {
            inner.get_token_configuration(token_id)
        } else {
            Ok(None)
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Return a reasonable default for platform activation height
        Ok(1)
    }
}

impl ContextProvider for WasmTrustedContext {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Delegate to the inner provider
        self.inner
            .get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        self.inner.get_data_contract(id, platform_version)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        self.inner.get_token_configuration(token_id)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        self.inner.get_platform_activation_height()
    }
}

impl WasmTrustedContext {
    pub fn new_mainnet() -> Result<Self, ContextProviderError> {
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            dash_sdk::dpp::dashcore::Network::Dash,
            None,
            std::num::NonZeroUsize::new(100).unwrap(),
        )
        .map_err(|e| ContextProviderError::Generic(e.to_string()))?
        .with_refetch_if_not_found(false); // Disable refetch since we'll pre-fetch

        Ok(Self {
            inner: std::sync::Arc::new(inner),
        })
    }

    pub fn new_testnet() -> Result<Self, ContextProviderError> {
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
            dash_sdk::dpp::dashcore::Network::Testnet,
            None,
            std::num::NonZeroUsize::new(100).unwrap(),
        )
        .map_err(|e| ContextProviderError::Generic(e.to_string()))?
        .with_refetch_if_not_found(false); // Disable refetch since we'll pre-fetch

        Ok(Self {
            inner: std::sync::Arc::new(inner),
        })
    }

    pub fn new_local() -> Result<Self, ContextProviderError> {
        Self::new_local_with_url("http://127.0.0.1:2444")
    }

    pub fn new_local_with_url(base_url: &str) -> Result<Self, ContextProviderError> {
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            dash_sdk::dpp::dashcore::Network::Regtest,
            base_url.to_string(),
            std::num::NonZeroUsize::new(100).unwrap(),
        )
        .map_err(|e| ContextProviderError::Generic(e.to_string()))?
        .with_refetch_if_not_found(false);

        Ok(Self {
            inner: std::sync::Arc::new(inner),
        })
    }

    /// Pre-fetch quorum information to populate the cache
    pub async fn prefetch_quorums(&self) -> Result<(), ContextProviderError> {
        self.inner.update_quorum_caches().await.map_err(|e| {
            ContextProviderError::Generic(format!("Failed to prefetch quorums: {}", e))
        })
    }

    pub async fn fetch_masternode_addresses(
        &self,
    ) -> Result<rs_dapi_client::AddressList, ContextProviderError> {
        let urls = self.inner.fetch_masternode_addresses().await.map_err(|e| {
            ContextProviderError::Generic(format!("Failed to fetch masternodes: {}", e))
        })?;

        let mut addresses = Vec::new();
        for url in urls {
            let uri = dash_sdk::sdk::Uri::from_maybe_shared(url.to_string()).map_err(|e| {
                ContextProviderError::Generic(format!("Invalid masternode URI '{}': {}", url, e))
            })?;
            let address = rs_dapi_client::Address::try_from(uri).map_err(|e| {
                ContextProviderError::Generic(format!(
                    "Invalid masternode address '{}': {}",
                    url, e
                ))
            })?;
            addresses.push(address);
        }

        Ok(rs_dapi_client::AddressList::from_iter(addresses))
    }

    /// Add a data contract to the known contracts cache
    pub fn add_known_contract(&self, contract: DataContract) {
        self.inner.add_known_contract(contract);
    }

    /// Remove a data contract from the known contracts cache (for cache invalidation)
    pub fn remove_known_contract(&self, id: &Identifier) {
        self.inner.remove_known_contract(id);
    }

    /// Add a token configuration to the known token configurations cache
    pub fn add_known_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        self.inner.add_known_token_configuration(token_id, config);
    }
}
