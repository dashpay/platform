use std::sync::Arc;

use dash_sdk::platform::ContextProvider;
use dash_sdk::{
    dpp::{data_contract::TokenConfiguration, prelude::CoreBlockHeight, version::PlatformVersion},
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use wasm_bindgen::prelude::wasm_bindgen;

use crate::error::WasmSdkError;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmContext {}

/// A wrapper for TrustedHttpContextProvider that works in WASM.
///
/// Holds pre-fetched quorum keys and discovered masternode addresses for
/// proof verification and network connectivity. Create one via the async
/// `prefetchMainnet()`, `prefetchTestnet()`, `prefetchDevnet()`, or
/// `prefetchLocal()` factory methods, then pass it to a builder via
/// `withTrustedContext()`.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmTrustedContext {
    inner: std::sync::Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>,
    discovered_addresses: Vec<rs_dapi_client::Address>,
}

impl ContextProvider for WasmContext {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Err(ContextProviderError::Generic(
            "Non-trusted mode is not supported in WASM. Please construct a WasmTrustedContext via prefetchMainnet/prefetchTestnet/prefetchDevnet/prefetchLocal and attach it with WasmSdkBuilder.withTrustedContext().".to_string()
        ))
    }

    fn get_data_contract(
        &self,
        _id: &Identifier,
        _platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // Return None for now - this means the contract will be fetched from the network
        Ok(None)
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // For WASM context without trusted provider, we need to fetch token configuration
        // from the network. This is a simplified implementation that would need to be
        // enhanced with actual network fetching logic in a production environment.
        // TODO: Implement actual token configuration fetching from network
        // For now, we'll return None which will cause the proof verification to fail
        // with a clearer error message indicating missing token configuration
        tracing::warn!(
            token_id = %token_id,
            "Token configuration not available in WASM context - this will cause proof verification to fail. Use trusted context builders for proof verification."
        );

        Ok(None)
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Return a reasonable default for platform activation height
        // This is the height at which Platform was activated on testnet
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

// JS-exported async factory methods
#[wasm_bindgen]
impl WasmTrustedContext {
    /// Pre-fetch quorum keys and masternode addresses for mainnet.
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.mainnet().withTrustedContext(context)`.
    #[wasm_bindgen(js_name = "prefetchMainnet")]
    pub async fn prefetch_mainnet() -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(dash_sdk::dpp::dashcore::Network::Mainnet, None, None).await
    }

    /// Pre-fetch quorum keys and masternode addresses for mainnet using a
    /// fully-specified quorum base URL (useful for testing against a staging
    /// or self-hosted quorums endpoint).
    #[wasm_bindgen(js_name = "prefetchMainnetWithUrl")]
    pub async fn prefetch_mainnet_with_url(
        base_url: String,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(
            dash_sdk::dpp::dashcore::Network::Mainnet,
            None,
            Some(base_url),
        )
        .await
    }

    /// Pre-fetch quorum keys and masternode addresses for testnet.
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.testnet().withTrustedContext(context)`.
    #[wasm_bindgen(js_name = "prefetchTestnet")]
    pub async fn prefetch_testnet() -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(dash_sdk::dpp::dashcore::Network::Testnet, None, None).await
    }

    /// Pre-fetch quorum keys and masternode addresses for testnet using a
    /// fully-specified quorum base URL (useful for testing against a staging
    /// or self-hosted quorums endpoint).
    #[wasm_bindgen(js_name = "prefetchTestnetWithUrl")]
    pub async fn prefetch_testnet_with_url(
        base_url: String,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(
            dash_sdk::dpp::dashcore::Network::Testnet,
            None,
            Some(base_url),
        )
        .await
    }

    /// Pre-fetch quorum keys and masternode addresses for a devnet.
    ///
    /// `devnet_name` is the short name of the devnet (e.g. `"paloma"`). The
    /// quorum base URL is derived as `https://quorums.<devnet_name>.networks.dash.org`.
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.newDevnet().withTrustedContext(context)`.
    #[wasm_bindgen(js_name = "prefetchDevnet")]
    pub async fn prefetch_devnet(devnet_name: String) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(
            dash_sdk::dpp::dashcore::Network::Devnet,
            Some(devnet_name),
            None,
        )
        .await
    }

    /// Pre-fetch quorum keys and masternode addresses for a devnet using a
    /// fully-specified quorum base URL.
    ///
    /// Use this when the default
    /// `https://quorums.<devnet_name>.networks.dash.org` URL produced by
    /// `prefetchDevnet` is not yet deployed for a devnet, or when pointing
    /// at a non-standard quorums endpoint.
    #[wasm_bindgen(js_name = "prefetchDevnetWithUrl")]
    pub async fn prefetch_devnet_with_url(
        base_url: String,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(
            dash_sdk::dpp::dashcore::Network::Devnet,
            None,
            Some(base_url),
        )
        .await
    }

    /// Pre-fetch quorum keys and masternode addresses for a local network.
    ///
    /// Uses the default local quorum sidecar URL (`http://127.0.0.1:22444`).
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.local().withTrustedContext(context)`.
    #[wasm_bindgen(js_name = "prefetchLocal")]
    pub async fn prefetch_local() -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_local_with_url("http://127.0.0.1:22444".to_string()).await
    }

    /// Pre-fetch quorum keys and masternode addresses for a local network
    /// using a custom quorum sidecar URL.
    #[wasm_bindgen(js_name = "prefetchLocalWithUrl")]
    pub async fn prefetch_local_with_url(
        base_url: String,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(
            dash_sdk::dpp::dashcore::Network::Regtest,
            None,
            Some(base_url),
        )
        .await
    }
}

impl WasmTrustedContext {
    /// Refresh quorum keys before proof verification.
    pub(crate) async fn refresh_quorums(&self) -> Result<(), WasmSdkError> {
        self.inner
            .refresh_quorum_caches()
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to refresh quorums: {}", e)))
    }

    /// Shared constructor used by every `prefetch*` factory. When `base_url`
    /// is `Some`, it overrides the default URL derived from `network` +
    /// `devnet_name` (the validator inside `new_with_url` still runs).
    async fn prefetch_for(
        network: dash_sdk::dpp::dashcore::Network,
        devnet_name: Option<String>,
        base_url: Option<String>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        let cache_size = std::num::NonZeroUsize::new(100).unwrap();
        let inner = match base_url {
            Some(url) => rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
                network, url, cache_size,
            ),
            None => rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new(
                network,
                devnet_name,
                cache_size,
            ),
        }
        .map_err(|e| WasmSdkError::generic(format!("Failed to create context provider: {}", e)))?
        .with_refetch_if_not_found(false);

        let inner = Arc::new(inner);

        inner
            .update_quorum_caches()
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to prefetch quorums: {}", e)))?;

        // Masternode discovery is an optional convenience: it only feeds the
        // no-explicit-addresses path in `withTrustedContext`, while the quorum
        // data prefetched above is what proof verification actually needs. On
        // a local network the sidecar's per-masternode version checks reject
        // the gateway's self-signed TLS, so discovery failing there is the
        // NORMAL case and must not make the whole trusted context unusable
        // for an SDK constructed with explicit addresses. On public networks
        // the failure stays fatal: it signals a genuine outage of the trusted
        // endpoint, and degrading silently would hide it.
        let discovered_addresses = match Self::fetch_addresses_from(&inner).await {
            Ok(addresses) => addresses,
            Err(e) if network == dash_sdk::dpp::dashcore::Network::Regtest => {
                tracing::warn!(
                    error = %e,
                    "trusted context: masternode discovery unavailable, continuing without \
                     discovered addresses (explicitly configured addresses are unaffected)"
                );
                Vec::new()
            }
            Err(e) => return Err(e),
        };

        Ok(WasmTrustedContext {
            inner,
            discovered_addresses,
        })
    }

    /// Fetch masternode addresses from the trusted provider and convert to `Vec<Address>`.
    async fn fetch_addresses_from(
        inner: &rs_sdk_trusted_context_provider::TrustedHttpContextProvider,
    ) -> Result<Vec<rs_dapi_client::Address>, WasmSdkError> {
        let urls = inner
            .fetch_masternode_addresses()
            .await
            .map_err(|e| WasmSdkError::generic(format!("Failed to fetch masternodes: {}", e)))?;

        let mut addresses = Vec::new();
        for url in urls {
            let uri = dash_sdk::sdk::Uri::from_maybe_shared(url.to_string()).map_err(|e| {
                WasmSdkError::generic(format!("Invalid masternode URI '{}': {}", url, e))
            })?;
            let address = rs_dapi_client::Address::try_from(uri).map_err(|e| {
                WasmSdkError::generic(format!("Invalid masternode address '{}': {}", url, e))
            })?;
            addresses.push(address);
        }

        Ok(addresses)
    }

    /// Get the discovered addresses (for use by the builder).
    pub(crate) fn discovered_addresses(&self) -> &[rs_dapi_client::Address] {
        &self.discovered_addresses
    }

    /// Add a data contract to the known contracts cache
    pub fn add_known_contract(&self, contract: DataContract) {
        self.inner.add_known_contract(contract);
    }

    /// Get a data contract from the known contracts cache
    pub fn get_known_contract(&self, id: &Identifier) -> Option<Arc<DataContract>> {
        self.inner.get_known_contract(id)
    }

    /// Remove a data contract from the known contracts cache
    pub fn remove_known_contract(&self, id: &Identifier) -> bool {
        self.inner.remove_known_contract(id)
    }

    /// Add a token configuration to the known token configurations cache
    pub fn add_known_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        self.inner.add_known_token_configuration(token_id, config);
    }

    /// Build a `WasmTrustedContext` with the given `discovered_addresses` for
    /// use in unit tests. The inner provider is constructed against a local
    /// loopback URL that these tests do not dial, providing a real
    /// `Arc<TrustedHttpContextProvider>` without network side effects.
    #[cfg(test)]
    pub(crate) fn for_testing(
        discovered_addresses: Vec<rs_dapi_client::Address>,
    ) -> WasmTrustedContext {
        Self::for_testing_with_url(discovered_addresses, "http://127.0.0.1:22444".to_string())
    }

    /// Build a test context whose provider reads from a controllable endpoint.
    #[cfg(test)]
    pub(crate) fn for_testing_with_url(
        discovered_addresses: Vec<rs_dapi_client::Address>,
        base_url: String,
    ) -> WasmTrustedContext {
        let cache_size = std::num::NonZeroUsize::new(1).unwrap();
        let inner = rs_sdk_trusted_context_provider::TrustedHttpContextProvider::new_with_url(
            dash_sdk::dpp::dashcore::Network::Regtest,
            base_url,
            cache_size,
        )
        .expect("test URL must construct a TrustedHttpContextProvider")
        .with_refetch_if_not_found(false);

        WasmTrustedContext {
            inner: Arc::new(inner),
            discovered_addresses,
        }
    }
}
