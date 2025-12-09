use std::sync::Arc;

use dash_sdk::platform::ContextProvider;
use dash_sdk::{
    dpp::{data_contract::TokenConfiguration, prelude::CoreBlockHeight, version::PlatformVersion},
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmContext {}

/// A wrapper for TrustedHttpContextProvider that works in WASM
#[derive(Clone)]
pub struct WasmTrustedContext {
    inner: std::sync::Arc<rs_sdk_trusted_context_provider::TrustedHttpContextProvider>,
}

impl ContextProvider for WasmContext {
    fn get_quorum_public_key(
        &self,
        _quorum_type: u32,
        _quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        Err(ContextProviderError::Generic(
            "Non-trusted mode is not supported in WASM. Please use the trusted SDK builders (new_mainnet_trusted or new_testnet_trusted) instead.".to_string()
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

    /// Pre-fetch quorum information to populate the cache
    pub async fn prefetch_quorums(&self) -> Result<(), ContextProviderError> {
        self.inner.update_quorum_caches().await.map_err(|e| {
            ContextProviderError::Generic(format!("Failed to prefetch quorums: {}", e))
        })
    }

    /// Add a data contract to the known contracts cache
    pub fn add_known_contract(&self, contract: DataContract) {
        self.inner.add_known_contract(contract);
    }
}
