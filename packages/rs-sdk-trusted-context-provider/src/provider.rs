use crate::error::TrustedContextProviderError;
use crate::get_quorum_base_url;
use crate::types::{PreviousQuorumsResponse, QuorumData, QuorumsResponse};

use arc_swap::ArcSwap;
use async_trait::async_trait;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
// QuorumHash is just [u8; 32]
type QuorumHash = [u8; 32];
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
use dpp::version::PlatformVersion;

/// Get the LLMQ type for the network
fn get_llmq_type_for_network(network: Network) -> u32 {
    match network {
        Network::Dash => 4,     // Mainnet uses LLMQ type 4
        Network::Testnet => 6,  // Testnet uses LLMQ type 6
        Network::Devnet => 107, // Devnet uses LLMQ type 107
        _ => 6,                 // Default to testnet type
    }
}
use lru::LruCache;
use reqwest::Client;
use std::collections::HashMap;
use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info};
use url::Url;

/// A trusted HTTP-based context provider that fetches quorum information
/// from trusted HTTP endpoints instead of requiring Core RPC access.
pub struct TrustedHttpContextProvider {
    network: Network,
    client: Client,
    base_url: String,

    /// Cache for current quorums
    current_quorums_cache: Arc<Mutex<LruCache<QuorumHash, QuorumData>>>,

    /// Cache for previous quorums
    previous_quorums_cache: Arc<Mutex<LruCache<QuorumHash, QuorumData>>>,

    /// Last fetched current quorums data
    last_current_quorums: Arc<ArcSwap<Option<QuorumsResponse>>>,

    /// Last fetched previous quorums data
    last_previous_quorums: Arc<ArcSwap<Option<PreviousQuorumsResponse>>>,

    /// Optional fallback provider for data contracts and token configurations
    fallback_provider: Option<Box<dyn ContextProvider>>,

    /// Known contracts cache - contracts that are pre-loaded and can be served immediately
    known_contracts: HashMap<Identifier, Arc<DataContract>>,
}

impl TrustedHttpContextProvider {
    /// Verify that a URL's domain resolves
    fn verify_domain_resolves(url: &str) -> Result<(), TrustedContextProviderError> {
        let parsed_url = Url::parse(url).map_err(|e| {
            TrustedContextProviderError::NetworkError(format!("Invalid URL: {}", e))
        })?;

        let host = parsed_url.host_str().ok_or_else(|| {
            TrustedContextProviderError::NetworkError("URL has no host".to_string())
        })?;

        let port = parsed_url.port_or_known_default().ok_or_else(|| {
            TrustedContextProviderError::NetworkError(
                "Unknown URL scheme and no port specified".to_string(),
            )
        })?;

        // Try to resolve the domain
        let addr = format!("{}:{}", host, port);
        match addr.to_socket_addrs() {
            Ok(mut addrs) => {
                if addrs.next().is_none() {
                    return Err(TrustedContextProviderError::NetworkError(format!(
                        "Domain '{}' does not resolve to any IP addresses",
                        host
                    )));
                }
                debug!("Domain '{}' resolves successfully", host);
                Ok(())
            }
            Err(e) => Err(TrustedContextProviderError::NetworkError(format!(
                "Failed to resolve domain '{}': {}",
                host, e
            ))),
        }
    }

    /// Create a new trusted HTTP context provider with default URLs
    pub fn new(
        network: Network,
        devnet_name: Option<String>,
        cache_size: NonZeroUsize,
    ) -> Result<Self, TrustedContextProviderError> {
        let base_url = get_quorum_base_url(network, devnet_name.as_deref())?;
        Self::new_with_url(network, base_url, cache_size)
    }

    /// Create a new trusted HTTP context provider with a custom URL
    pub fn new_with_url(
        network: Network,
        base_url: String,
        cache_size: NonZeroUsize,
    ) -> Result<Self, TrustedContextProviderError> {
        // Verify the domain resolves before proceeding
        Self::verify_domain_resolves(&base_url)?;

        let client = Client::builder().timeout(Duration::from_secs(30)).build()?;

        Ok(Self {
            network,
            client,
            base_url,
            current_quorums_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            previous_quorums_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            last_current_quorums: Arc::new(ArcSwap::new(Arc::new(None))),
            last_previous_quorums: Arc::new(ArcSwap::new(Arc::new(None))),
            fallback_provider: None,
            known_contracts: HashMap::new(),
        })
    }

    /// Set a fallback provider for data contracts and token configurations
    pub fn with_fallback_provider<P: ContextProvider + 'static>(mut self, provider: P) -> Self {
        self.fallback_provider = Some(Box::new(provider));
        self
    }

    /// Set known contracts that will be served immediately without fallback
    pub fn with_known_contracts(mut self, contracts: Vec<DataContract>) -> Self {
        for contract in contracts {
            let id = contract.id();
            self.known_contracts.insert(id, Arc::new(contract));
        }
        self
    }

    /// Fetch current quorums from the HTTP endpoint
    async fn fetch_current_quorums(&self) -> Result<QuorumsResponse, TrustedContextProviderError> {
        let llmq_type = get_llmq_type_for_network(self.network);
        let url = format!("{}/quorums?quorumType={}", self.base_url, llmq_type);
        debug!("Fetching current quorums from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(TrustedContextProviderError::NetworkError(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let quorums: QuorumsResponse = response.json().await?;

        // Update cache
        self.last_current_quorums
            .store(Arc::new(Some(quorums.clone())));

        // Cache individual quorums
        if let Ok(mut cache) = self.current_quorums_cache.lock() {
            for quorum in &quorums.data {
                match hex::decode(&quorum.quorum_hash)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok())
                {
                    Some(hash) => {
                        cache.put(hash, quorum.clone());
                    }
                    None => {
                        debug!(
                            "Skipping invalid quorum hash '{}' for current quorums",
                            quorum.quorum_hash
                        );
                    }
                }
            }
        }

        Ok(quorums)
    }

    /// Fetch previous quorums from the HTTP endpoint
    async fn fetch_previous_quorums(
        &self,
    ) -> Result<PreviousQuorumsResponse, TrustedContextProviderError> {
        let llmq_type = get_llmq_type_for_network(self.network);
        let url = format!("{}/previous?quorumType={}", self.base_url, llmq_type);
        debug!("Fetching previous quorums from: {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(TrustedContextProviderError::NetworkError(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let quorums: PreviousQuorumsResponse = response.json().await?;

        // Update cache
        self.last_previous_quorums
            .store(Arc::new(Some(quorums.clone())));

        // Cache individual quorums
        if let Ok(mut cache) = self.previous_quorums_cache.lock() {
            for quorum in &quorums.data.quorums {
                match hex::decode(&quorum.quorum_hash)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok())
                {
                    Some(hash) => {
                        cache.put(hash, quorum.clone());
                    }
                    None => {
                        debug!(
                            "Skipping invalid quorum hash '{}' for previous quorums",
                            quorum.quorum_hash
                        );
                    }
                }
            }
        }

        Ok(quorums)
    }

    /// Find a quorum by type and hash
    async fn find_quorum(
        &self,
        quorum_type: u32,
        quorum_hash: QuorumHash,
    ) -> Result<QuorumData, TrustedContextProviderError> {
        let expected_type = get_llmq_type_for_network(self.network);
        if quorum_type != expected_type {
            debug!(
                "Quorum type {} doesn't match network type {}",
                quorum_type, expected_type
            );
        }

        // Check current cache first
        if let Ok(mut cache) = self.current_quorums_cache.lock() {
            if let Some(quorum) = cache.get(&quorum_hash) {
                debug!("Found quorum in current cache");
                return Ok(quorum.clone());
            }
        }

        // Check previous cache
        if let Ok(mut cache) = self.previous_quorums_cache.lock() {
            if let Some(quorum) = cache.get(&quorum_hash) {
                debug!("Found quorum in previous cache");
                return Ok(quorum.clone());
            }
        }

        // Fetch fresh data
        info!("Quorum not in cache, fetching fresh data");

        // Try current quorums first
        if let Ok(current) = self.fetch_current_quorums().await {
            for quorum in &current.data {
                let hash_bytes: Option<[u8; 32]> = hex::decode(&quorum.quorum_hash)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok());

                if let Some(hash_bytes) = hash_bytes {
                    if hash_bytes == quorum_hash {
                        return Ok(quorum.clone());
                    }
                }
            }
        }

        // Try previous quorums
        if let Ok(previous) = self.fetch_previous_quorums().await {
            for quorum in &previous.data.quorums {
                let hash_bytes: Option<[u8; 32]> = hex::decode(&quorum.quorum_hash)
                    .ok()
                    .and_then(|bytes| bytes.try_into().ok());

                if let Some(hash_bytes) = hash_bytes {
                    if hash_bytes == quorum_hash {
                        return Ok(quorum.clone());
                    }
                }
            }
        }

        Err(TrustedContextProviderError::QuorumNotFound {
            quorum_type,
            quorum_hash: hex::encode(quorum_hash),
        })
    }
}

#[async_trait]
impl ContextProvider for TrustedHttpContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: QuorumHash,
        _core_chain_locked_height: CoreBlockHeight,
    ) -> Result<[u8; 48], ContextProviderError> {
        // Use blocking to run async code in sync context
        let quorum = futures::executor::block_on(self.find_quorum(quorum_type, quorum_hash))
            .map_err(|e| ContextProviderError::Generic(e.to_string()))?;

        // Parse the public key from the 'key' field
        let pubkey_hex = quorum.key.trim_start_matches("0x");
        let pubkey_bytes = hex::decode(pubkey_hex).map_err(|e| {
            ContextProviderError::Generic(format!("Invalid hex in public key: {}", e))
        })?;

        if pubkey_bytes.len() != 48 {
            return Err(ContextProviderError::Generic(format!(
                "Invalid public key length: {} bytes, expected 48",
                pubkey_bytes.len()
            )));
        }

        pubkey_bytes.try_into().map_err(|_| {
            ContextProviderError::Generic("Failed to convert public key to array".to_string())
        })
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // First check known contracts cache
        if let Some(contract) = self.known_contracts.get(id) {
            return Ok(Some(contract.clone()));
        }

        // If not found in known contracts, delegate to fallback provider if available
        if let Some(ref provider) = self.fallback_provider {
            provider.get_data_contract(id, platform_version)
        } else {
            // No fallback provider, return None
            Ok(None)
        }
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // Delegate to fallback provider if available
        if let Some(ref provider) = self.fallback_provider {
            provider.get_token_configuration(token_id)
        } else {
            // No fallback provider, return None
            Ok(None)
        }
    }

    fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
        // Return the L1 locked height for each network
        match self.network {
            Network::Dash => Ok(2132092),    // Mainnet L1 locked height
            Network::Testnet => Ok(1090319), // Testnet L1 locked height
            Network::Devnet => Ok(1),        // Devnet activation height
            _ => Err(ContextProviderError::Generic(
                "Unsupported network".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_quorum_base_url() {
        assert_eq!(
            get_quorum_base_url(Network::Dash, None).unwrap(),
            "https://quorums.mainnet.networks.dash.org"
        );

        assert_eq!(
            get_quorum_base_url(Network::Testnet, None).unwrap(),
            "https://quorums.testnet.networks.dash.org"
        );

        assert_eq!(
            get_quorum_base_url(Network::Devnet, Some("example")).unwrap(),
            "https://quorums.devnet.example.networks.dash.org"
        );
    }

    #[test]
    fn test_devnet_without_name_returns_error() {
        let result = get_quorum_base_url(Network::Devnet, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TrustedContextProviderError::InvalidDevnetName(_)
        ));
    }

    #[test]
    fn test_regtest_returns_error() {
        let result = get_quorum_base_url(Network::Regtest, None);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            TrustedContextProviderError::UnsupportedNetwork(_)
        ));
    }

    #[test]
    fn test_invalid_devnet_names() {
        // Empty name
        let result = get_quorum_base_url(Network::Devnet, Some(""));
        assert!(result.is_err());

        // Name with special characters
        let result = get_quorum_base_url(Network::Devnet, Some("test@name"));
        assert!(result.is_err());

        // Name starting with hyphen
        let result = get_quorum_base_url(Network::Devnet, Some("-test"));
        assert!(result.is_err());

        // Name ending with hyphen
        let result = get_quorum_base_url(Network::Devnet, Some("test-"));
        assert!(result.is_err());

        // Valid names should work
        assert!(get_quorum_base_url(Network::Devnet, Some("test")).is_ok());
        assert!(get_quorum_base_url(Network::Devnet, Some("test-123")).is_ok());
        assert!(get_quorum_base_url(Network::Devnet, Some("TEST123")).is_ok());
    }

    #[test]
    fn test_known_contracts() {
        use dpp::version::PlatformVersion;

        // Create a provider
        let provider = TrustedHttpContextProvider::new(
            Network::Testnet,
            None,
            NonZeroUsize::new(100).unwrap(),
        )
        .unwrap();

        // Test that initially there are no known contracts
        let contract_id = Identifier::from([1u8; 32]);
        let retrieved = provider
            .get_data_contract(&contract_id, PlatformVersion::latest())
            .unwrap();
        assert!(retrieved.is_none());

        // Test that we can use the builder pattern to add known contracts
        // The builder pattern is more appropriate since contracts are only added during initialization
    }

    #[test]
    fn test_domain_resolution_check() {
        // Test with a domain that should resolve (using localhost)
        let result = TrustedHttpContextProvider::verify_domain_resolves("https://localhost");
        assert!(result.is_ok());

        // Test with HTTP URL (should use port 80 by default)
        let result = TrustedHttpContextProvider::verify_domain_resolves("http://localhost");
        assert!(result.is_ok());

        // Test with an invalid domain that won't resolve
        let result = TrustedHttpContextProvider::verify_domain_resolves(
            "https://this-domain-definitely-does-not-exist-12345.com",
        );
        assert!(result.is_err());

        // Test with an invalid URL
        let result = TrustedHttpContextProvider::verify_domain_resolves("not-a-valid-url");
        assert!(result.is_err());

        // Test with unknown scheme - should fail due to port_or_known_default returning None
        let result = TrustedHttpContextProvider::verify_domain_resolves("unknown://localhost");
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_creation_with_invalid_domain() {
        // This test will fail if we try to create a provider with an invalid devnet name
        // that results in a non-resolving domain
        let result = TrustedHttpContextProvider::new(
            Network::Devnet,
            Some("nonexistent-devnet-12345".to_string()),
            NonZeroUsize::new(100).unwrap(),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_provider_with_custom_url() {
        // Test with a valid custom URL (localhost should resolve)
        let result = TrustedHttpContextProvider::new_with_url(
            Network::Testnet,
            "https://localhost:8080".to_string(),
            NonZeroUsize::new(100).unwrap(),
        );
        assert!(result.is_ok());

        let provider = result.unwrap();
        assert_eq!(provider.base_url, "https://localhost:8080");

        // Test with an invalid custom URL
        let result = TrustedHttpContextProvider::new_with_url(
            Network::Testnet,
            "https://this-domain-definitely-does-not-exist-12345.com".to_string(),
            NonZeroUsize::new(100).unwrap(),
        );
        assert!(result.is_err());
    }
}
