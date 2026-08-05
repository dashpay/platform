use crate::error::TrustedContextProviderError;
use crate::get_quorum_base_url;
use crate::types::{PreviousQuorumsResponse, QuorumData, QuorumsResponse};

use arc_swap::ArcSwap;
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::prelude::{CoreBlockHeight, DataContract, Identifier};
// QuorumHash is just [u8; 32]
type QuorumHash = [u8; 32];
use dpp::dashcore::Network;
use dpp::data_contract::TokenConfiguration;
#[cfg(any(
    feature = "dpns-contract",
    feature = "dashpay-contract",
    feature = "withdrawals-contract",
    feature = "wallet-utils-contract",
    feature = "token-history-contract",
    feature = "keywords-contract",
    feature = "document-history-contract",
    feature = "all-system-contracts"
))]
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;

use lru::LruCache;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error as StdError;
#[cfg(all(
    not(target_arch = "wasm32"),
    not(any(target_os = "ios", target_os = "android"))
))]
use std::net::ToSocketAddrs;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::{debug, info};
use url::Url;

#[cfg(target_arch = "wasm32")]
const WASM_HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// A trusted HTTP-based context provider that fetches quorum information
/// from trusted HTTP endpoints instead of requiring Core RPC access.
#[derive(Clone)]
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
    fallback_provider: Option<Arc<dyn ContextProvider>>,

    /// Known contracts cache - contracts that are pre-loaded and can be served immediately
    known_contracts: Arc<Mutex<HashMap<Identifier, Arc<DataContract>>>>,

    /// Known token configurations cache - token configs that are pre-loaded for proof verification
    known_token_configurations: Arc<Mutex<HashMap<Identifier, TokenConfiguration>>>,

    /// Whether to refetch quorums if not found in cache
    refetch_if_not_found: bool,
}

#[derive(Debug, Deserialize)]
struct MasternodeEntry {
    address: String,
    status: String,
    #[serde(rename = "versionCheck")]
    version_check: Option<String>,
    #[serde(rename = "platformHTTPPort")]
    platform_http_port: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct MasternodeDiscoveryResponse {
    success: bool,
    data: Vec<MasternodeEntry>,
}

impl TrustedHttpContextProvider {
    /// Build a GET request for a trusted endpoint.
    ///
    /// Every request to `base_url` goes through here. On wasm32 the client
    /// carries no timeout, because `ClientBuilder::timeout` is unavailable
    /// there, so each request sets its own; a slow or hung endpoint would
    /// otherwise block the caller forever. Cached responses are bypassed so a
    /// rotated quorum or a changed masternode list is actually observed.
    fn http_request(&self, url: &str) -> reqwest::RequestBuilder {
        let request = self.client.get(url);

        #[cfg(target_arch = "wasm32")]
        let request = request
            .timeout(WASM_HTTP_REQUEST_TIMEOUT)
            .fetch_cache_no_store();

        request
    }

    /// Verify that a URL's domain resolves
    #[cfg(all(
        not(target_arch = "wasm32"),
        not(any(target_os = "ios", target_os = "android"))
    ))]
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
        // The base URL is the SDK's root of trust for proof verification
        // (quorum public keys) and network connectivity (discovered masternode
        // addresses). For production networks (mainnet/testnet) there is no
        // legitimate plaintext workflow — HTTPS is deployed — so refuse to
        // hand a non-TLS quorum URL to the SDK. Devnet/Regtest keep the
        // plaintext escape hatch (early-stage devnets without a cert yet,
        // local sidecars on loopback).
        if matches!(network, Network::Mainnet | Network::Testnet) {
            let parsed = Url::parse(&base_url).map_err(|e| {
                TrustedContextProviderError::NetworkError(format!("Invalid URL: {}", e))
            })?;
            if !parsed.scheme().eq_ignore_ascii_case("https") {
                return Err(TrustedContextProviderError::NetworkError(format!(
                    "Custom quorum URL for {:?} must use https://; got '{}'",
                    network, base_url
                )));
            }
        }

        // Verify the domain resolves before proceeding (skip on WASM and iOS)
        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(target_os = "ios", target_os = "android"))
        ))]
        Self::verify_domain_resolves(&base_url)?;

        #[cfg(target_arch = "wasm32")]
        let client = Client::builder().build()?;

        #[cfg(all(not(target_arch = "wasm32"), target_os = "ios"))]
        let client = {
            // iOS specific configuration
            Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("DashSDK-iOS/1.0")
                .build()?
        };

        #[cfg(all(not(target_arch = "wasm32"), target_os = "android"))]
        let client = {
            // Android: no blocking DNS pre-check, platform-labelled UA
            Client::builder()
                .timeout(Duration::from_secs(30))
                .user_agent("DashSDK-Android/1.0")
                .build()?
        };

        #[cfg(all(
            not(target_arch = "wasm32"),
            not(any(target_os = "ios", target_os = "android"))
        ))]
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("DashSDK/1.0")
            .build()?;

        Ok(Self {
            network,
            client,
            base_url,
            current_quorums_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            previous_quorums_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            last_current_quorums: Arc::new(ArcSwap::new(Arc::new(None))),
            last_previous_quorums: Arc::new(ArcSwap::new(Arc::new(None))),
            fallback_provider: None,
            known_contracts: Arc::new(Mutex::new(HashMap::new())),
            known_token_configurations: Arc::new(Mutex::new(HashMap::new())),
            refetch_if_not_found: true,
        })
    }

    /// Set a fallback provider for data contracts and token configurations
    pub fn with_fallback_provider<P: ContextProvider + 'static>(mut self, provider: P) -> Self {
        self.fallback_provider = Some(Arc::new(provider));
        self
    }

    /// Set known contracts that will be served immediately without fallback
    pub fn with_known_contracts(self, contracts: Vec<DataContract>) -> Self {
        let mut known = self.known_contracts.lock().unwrap();
        for contract in contracts {
            let id = contract.id();
            known.insert(id, Arc::new(contract));
        }
        drop(known);
        self
    }

    /// Set whether to refetch quorums if not found in cache
    pub fn with_refetch_if_not_found(mut self, refetch: bool) -> Self {
        self.refetch_if_not_found = refetch;
        self
    }

    /// Add a data contract to the known contracts cache
    pub fn add_known_contract(&self, contract: DataContract) {
        let id = contract.id();
        let mut known = self.known_contracts.lock().unwrap();
        known.insert(id, Arc::new(contract));
    }

    /// Get a data contract from the known contracts cache
    /// Returns None if the contract is not in the cache
    pub fn get_known_contract(&self, id: &Identifier) -> Option<Arc<DataContract>> {
        let known = self.known_contracts.lock().unwrap();
        known.get(id).cloned()
    }

    /// Remove a data contract from the known contracts cache
    /// Returns true if the contract was present and removed, false otherwise
    pub fn remove_known_contract(&self, id: &Identifier) -> bool {
        let mut known = self.known_contracts.lock().unwrap();
        known.remove(id).is_some()
    }

    /// Add multiple data contracts to the known contracts cache
    pub fn add_known_contracts(&self, contracts: Vec<DataContract>) {
        let mut known = self.known_contracts.lock().unwrap();
        for contract in contracts {
            let id = contract.id();
            known.insert(id, Arc::new(contract));
        }
    }

    /// Add a token configuration to the known token configurations cache
    pub fn add_known_token_configuration(&self, token_id: Identifier, config: TokenConfiguration) {
        let mut known = self.known_token_configurations.lock().unwrap();
        known.insert(token_id, config);
    }

    /// Add multiple token configurations to the known token configurations cache
    pub fn add_known_token_configurations(&self, configs: Vec<(Identifier, TokenConfiguration)>) {
        let mut known = self.known_token_configurations.lock().unwrap();
        for (token_id, config) in configs {
            known.insert(token_id, config);
        }
    }

    /// Update the quorum caches by fetching current and previous quorums
    pub async fn update_quorum_caches(&self) -> Result<(), TrustedContextProviderError> {
        // Fetch current quorums
        let current = self.fetch_current_quorums().await?;

        // Fetch previous quorums
        let previous = self.fetch_previous_quorums().await?;

        // The caches are already updated by the fetch methods
        debug!(
            "Successfully updated quorum caches with {} current and {} previous quorums",
            current.data.len(),
            previous.data.quorums.len()
        );

        Ok(())
    }

    /// Refresh current and previous quorum caches independently.
    ///
    /// Both endpoints are attempted so a failure from one does not prevent
    /// usable data from the other from reaching the caches.
    pub async fn refresh_quorum_caches(&self) -> Result<(), TrustedContextProviderError> {
        let current_error = self.fetch_current_quorums().await.err();
        let previous_error = self.fetch_previous_quorums().await.err();

        match (current_error, previous_error) {
            (None, None) => Ok(()),
            (Some(error), None) => Err(TrustedContextProviderError::NetworkError(format!(
                "Failed to refresh current quorums: {}",
                error
            ))),
            (None, Some(error)) => Err(TrustedContextProviderError::NetworkError(format!(
                "Failed to refresh previous quorums: {}",
                error
            ))),
            (Some(current_error), Some(previous_error)) => {
                Err(TrustedContextProviderError::NetworkError(format!(
                    "Failed to refresh current quorums: {}; failed to refresh previous quorums: {}",
                    current_error, previous_error
                )))
            }
        }
    }

    /// Get the total number of quorums in both caches
    pub fn get_cached_quorum_count(&self) -> usize {
        let current_count = self
            .current_quorums_cache
            .lock()
            .map(|cache| cache.len())
            .unwrap_or(0);

        let previous_count = self
            .previous_quorums_cache
            .lock()
            .map(|cache| cache.len())
            .unwrap_or(0);

        current_count + previous_count
    }

    /// Fetch DAPI HTTPS addresses from the masternode discovery endpoint
    pub async fn fetch_masternode_addresses(
        &self,
    ) -> Result<Vec<Url>, TrustedContextProviderError> {
        let url = format!("{}/masternodes", self.base_url);
        debug!(
            "Fetching masternode addresses from trusted resource: {}",
            url
        );

        let response = self.http_request(&url).send().await?;
        if !response.status().is_success() {
            return Err(TrustedContextProviderError::NetworkError(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        let body = response.text().await?;
        let parsed: MasternodeDiscoveryResponse = serde_json::from_str(&body)?;

        if !parsed.success {
            return Err(TrustedContextProviderError::NetworkError(
                "Masternode discovery response indicated failure".to_string(),
            ));
        }

        let default_dapi_port = match self.network {
            Network::Mainnet => 443,
            Network::Testnet => 1443,
            _ => 443,
        };

        let mut addresses = Vec::new();
        for entry in parsed
            .data
            .into_iter()
            .filter(|m| m.status == "ENABLED" && m.version_check.as_deref() == Some("success"))
        {
            let host_port = entry.address;
            let host = host_port
                .rsplit_once(':')
                .map(|(h, _)| h)
                .unwrap_or(host_port.as_str());
            // Use platformHTTPPort from the entry if available, otherwise use network default
            let dapi_port = entry.platform_http_port.unwrap_or(default_dapi_port);
            let https_url = format!("https://{}:{}", host, dapi_port);
            let url = url::Url::parse(&https_url).map_err(|e| {
                TrustedContextProviderError::NetworkError(format!(
                    "Invalid masternode URL '{}': {}",
                    https_url, e
                ))
            })?;
            addresses.push(url);
        }

        if addresses.is_empty() {
            return Err(TrustedContextProviderError::NetworkError(
                "No eligible masternode addresses discovered".to_string(),
            ));
        }

        Ok(addresses)
    }

    /// Fetch current quorums from the HTTP endpoint
    pub async fn fetch_current_quorums(
        &self,
    ) -> Result<QuorumsResponse, TrustedContextProviderError> {
        let url = format!("{}/quorums", self.base_url);
        debug!("Fetching current quorums from: {}", url);

        let response = match self.http_request(&url).send().await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::error!(error = ?e, url = %url, "HTTP request failed");
                if let Some(source) = e.source() {
                    tracing::error!(?source, "Error source");
                    if let Some(inner) = source.source() {
                        tracing::error!(?inner, "Inner error");
                    }
                }

                // Check for specific error types (connect detection not available across all reqwest versions)
                if e.is_timeout() {
                    tracing::error!("Request timeout");
                } else if e.is_request() {
                    tracing::error!("Error building the request");
                } else if e.is_body() {
                    tracing::error!("Error reading response body");
                } else if e.is_decode() {
                    tracing::error!("Error decoding response");
                }

                return Err(e.into());
            }
        };
        debug!("Received response with status: {}", response.status());

        if !response.status().is_success() {
            return Err(TrustedContextProviderError::NetworkError(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        debug!("Parsing JSON response for current quorums");
        let quorums: QuorumsResponse = response.json().await?;
        if !quorums.success {
            return Err(TrustedContextProviderError::NetworkError(
                "Current quorum response indicated failure".to_string(),
            ));
        }
        debug!("Successfully parsed {} quorums", quorums.data.len());

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
    pub async fn fetch_previous_quorums(
        &self,
    ) -> Result<PreviousQuorumsResponse, TrustedContextProviderError> {
        let url = format!("{}/previous", self.base_url);
        debug!("Fetching previous quorums from: {}", url);

        let response = self.http_request(&url).send().await?;
        debug!("Received response with status: {}", response.status());

        if !response.status().is_success() {
            return Err(TrustedContextProviderError::NetworkError(format!(
                "HTTP {} from {}",
                response.status(),
                url
            )));
        }

        debug!("Parsing JSON response for previous quorums");
        let quorums: PreviousQuorumsResponse = response.json().await?;
        if !quorums.success {
            return Err(TrustedContextProviderError::NetworkError(
                "Previous quorum response indicated failure".to_string(),
            ));
        }
        debug!(
            "Successfully parsed {} previous quorums",
            quorums.data.quorums.len()
        );

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

        // Check if we should refetch
        if !self.refetch_if_not_found {
            return Err(TrustedContextProviderError::QuorumNotFound {
                quorum_type,
                quorum_hash: hex::encode(quorum_hash),
            });
        }

        // Fetch fresh data
        info!(
            "Quorum not in cache, fetching fresh data for hash: {}",
            hex::encode(quorum_hash)
        );

        // Try current quorums first
        debug!("Attempting to fetch current quorums");
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
        } else {
            debug!("Failed to fetch current quorums");
        }

        // Try previous quorums
        debug!("Attempting to fetch previous quorums");
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

    /// Parse a BLS quorum public key from its hex representation.
    ///
    /// Accepts an optional `0x` prefix. Returns `ContextProviderError::Generic`
    /// on invalid hex, wrong length (must be 48 bytes), or conversion failure.
    fn parse_quorum_public_key(key: &str) -> Result<[u8; 48], ContextProviderError> {
        let pubkey_hex = key
            .strip_prefix("0x")
            .or_else(|| key.strip_prefix("0X"))
            .unwrap_or(key);
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
}

impl ContextProvider for TrustedHttpContextProvider {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: QuorumHash,
        _core_chain_locked_height: CoreBlockHeight,
    ) -> Result<[u8; 48], ContextProviderError> {
        debug!(
            "get_quorum_public_key called for type {} hash {}",
            quorum_type,
            hex::encode(quorum_hash)
        );

        // Check current cache first
        if let Ok(mut cache) = self.current_quorums_cache.lock() {
            if let Some(quorum) = cache.get(&quorum_hash) {
                debug!("Found quorum in current cache");
                return Self::parse_quorum_public_key(&quorum.key);
            }
        }

        // Check previous cache
        if let Ok(mut cache) = self.previous_quorums_cache.lock() {
            if let Some(quorum) = cache.get(&quorum_hash) {
                debug!("Found quorum in previous cache");
                return Self::parse_quorum_public_key(&quorum.key);
            }
        }

        // If not in cache and refetch is disabled, return error
        if !self.refetch_if_not_found {
            return Err(ContextProviderError::InvalidQuorum(format!(
                "Quorum not found in cache for hash: {}",
                hex::encode(quorum_hash)
            )));
        }

        let this = self.clone();
        let quorum =
            dash_async::block_on(async move { this.find_quorum(quorum_type, quorum_hash).await })?
                .map_err(|e| {
                    debug!("Error finding quorum: {}", e);
                    ContextProviderError::Generic(format!("Failed to find quorum: {}", e))
                })?;

        Self::parse_quorum_public_key(&quorum.key)
    }

    fn get_data_contract(
        &self,
        id: &Identifier,
        platform_version: &PlatformVersion,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        // First check known contracts cache
        let known = self.known_contracts.lock().unwrap();
        if let Some(contract) = known.get(id) {
            return Ok(Some(contract.clone()));
        }
        drop(known);

        // Check if this is a system data contract and the corresponding feature is enabled
        #[cfg(any(
            feature = "dpns-contract",
            feature = "dashpay-contract",
            feature = "withdrawals-contract",
            feature = "wallet-utils-contract",
            feature = "token-history-contract",
            feature = "keywords-contract",
            feature = "document-history-contract",
            feature = "all-system-contracts"
        ))]
        {
            // Check each system contract if its feature is enabled
            #[cfg(any(feature = "dpns-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::DPNS.id() {
                return load_system_data_contract(SystemDataContract::DPNS, platform_version)
                    .map(|contract| Some(Arc::new(contract)))
                    .map_err(|e| {
                        ContextProviderError::Generic(format!(
                            "Failed to load DPNS contract: {}",
                            e
                        ))
                    });
            }

            #[cfg(any(feature = "dashpay-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::Dashpay.id() {
                return load_system_data_contract(SystemDataContract::Dashpay, platform_version)
                    .map(|contract| Some(Arc::new(contract)))
                    .map_err(|e| {
                        ContextProviderError::Generic(format!(
                            "Failed to load Dashpay contract: {}",
                            e
                        ))
                    });
            }

            #[cfg(any(feature = "withdrawals-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::Withdrawals.id() {
                return load_system_data_contract(
                    SystemDataContract::Withdrawals,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load Withdrawals contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "wallet-utils-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::WalletUtils.id() {
                return load_system_data_contract(
                    SystemDataContract::WalletUtils,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load WalletUtils contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "token-history-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::TokenHistory.id() {
                return load_system_data_contract(
                    SystemDataContract::TokenHistory,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load TokenHistory contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(feature = "keywords-contract", feature = "all-system-contracts"))]
            if *id == SystemDataContract::KeywordSearch.id() {
                return load_system_data_contract(
                    SystemDataContract::KeywordSearch,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load KeywordSearch contract: {}",
                        e
                    ))
                });
            }

            #[cfg(any(
                feature = "document-history-contract",
                feature = "all-system-contracts"
            ))]
            if *id == SystemDataContract::DocumentHistory.id() {
                return load_system_data_contract(
                    SystemDataContract::DocumentHistory,
                    platform_version,
                )
                .map(|contract| Some(Arc::new(contract)))
                .map_err(|e| {
                    ContextProviderError::Generic(format!(
                        "Failed to load DocumentHistory contract: {}",
                        e
                    ))
                });
            }
        }

        // If not found in known contracts or system contracts, delegate to fallback provider if available
        if let Some(ref provider) = self.fallback_provider {
            provider.get_data_contract(id, platform_version)
        } else {
            // No fallback provider, return None
            Ok(None)
        }
    }

    fn register_data_contract(&self, contract: Arc<DataContract>) {
        let id = contract.id();
        let mut known = self.known_contracts.lock().unwrap();
        known.insert(id, contract);
    }

    fn get_token_configuration(
        &self,
        token_id: &Identifier,
    ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
        // First check known token configurations cache
        let known = self.known_token_configurations.lock().unwrap();
        if let Some(config) = known.get(token_id) {
            return Ok(Some(config.clone()));
        }
        drop(known);

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
            Network::Mainnet => Ok(2132092), // Mainnet L1 locked height
            Network::Testnet => Ok(1090319), // Testnet L1 locked height
            Network::Devnet | Network::Regtest => Ok(1), // Devnet/Regtest activation height
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::time::{Duration, Instant};

    fn accept_before(listener: &TcpListener, deadline: Instant) -> TcpStream {
        loop {
            match listener.accept() {
                Ok((stream, _)) => return stream,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("accept quorum request: {}", error),
            }
        }
    }

    fn current_response(hash: u8, key: u8) -> String {
        serde_json::json!({
            "success": true,
            "data": [{
                "quorum_hash": hex::encode([hash; 32]),
                "key": hex::encode([key; 48]),
                "height": 1,
                "valid_members_count": 3
            }]
        })
        .to_string()
    }

    fn empty_current_response() -> String {
        serde_json::json!({
            "success": true,
            "data": []
        })
        .to_string()
    }

    fn previous_response(hash: u8, key: u8) -> String {
        serde_json::json!({
            "success": true,
            "data": {
                "height": 1,
                "quorums": [{
                    "quorum_hash": hex::encode([hash; 32]),
                    "key": hex::encode([key; 48]),
                    "height": 1,
                    "valid_members_count": 3
                }]
            }
        })
        .to_string()
    }

    fn empty_previous_response() -> String {
        serde_json::json!({
            "success": true,
            "data": {
                "height": 1,
                "quorums": []
            }
        })
        .to_string()
    }

    fn spawn_http_responses(
        responses: Vec<(&str, u16, String)>,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock quorum endpoint");
        listener
            .set_nonblocking(true)
            .expect("make mock endpoint bounded");
        let address = listener.local_addr().expect("read mock endpoint address");
        let responses = responses
            .into_iter()
            .map(|(path, status, body)| (path.to_string(), status, body))
            .collect::<Vec<_>>();

        let handle = thread::spawn(move || {
            for (expected_path, status, body) in responses {
                let mut stream = accept_before(&listener, Instant::now() + Duration::from_secs(5));
                let mut reader =
                    BufReader::new(stream.try_clone().expect("clone quorum request stream"));
                let mut request_line = String::new();
                reader
                    .read_line(&mut request_line)
                    .expect("read quorum request line");
                assert_eq!(
                    request_line.split_whitespace().nth(1),
                    Some(expected_path.as_str())
                );

                loop {
                    let mut header = String::new();
                    reader
                        .read_line(&mut header)
                        .expect("read quorum request header");
                    if header == "\r\n" || header.is_empty() {
                        break;
                    }
                }

                let reason = if status == 200 {
                    "OK"
                } else {
                    "Internal Server Error"
                };
                write!(
                    stream,
                    "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    status,
                    reason,
                    body.len(),
                    body
                )
                .expect("write quorum response");
                stream.flush().expect("flush quorum response");
            }
        });

        (format!("http://{}", address), handle)
    }

    fn provider_for(base_url: String) -> TrustedHttpContextProvider {
        TrustedHttpContextProvider::new_with_url(
            Network::Regtest,
            base_url,
            NonZeroUsize::new(100).unwrap(),
        )
        .expect("construct mock trusted context provider")
        .with_refetch_if_not_found(false)
    }

    #[tokio::test]
    async fn refresh_quorum_caches_makes_rotated_keys_available() {
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 200, current_response(0x11, 0x41)),
            ("/previous", 200, previous_response(0x12, 0x42)),
        ]);
        let provider = provider_for(base_url);

        assert!(matches!(
            provider
                .get_quorum_public_key(1, [0x11; 32], 1)
                .expect_err("rotated quorum must be absent before refresh"),
            ContextProviderError::InvalidQuorum(_)
        ));
        provider
            .refresh_quorum_caches()
            .await
            .expect("both quorum endpoints must refresh");
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x11; 32], 1)
                .expect("current quorum must be cached"),
            [0x41; 48]
        );
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x12; 32], 1)
                .expect("previous quorum must be cached"),
            [0x42; 48]
        );
        server.join().expect("mock quorum server must finish");
    }

    #[tokio::test]
    async fn refresh_quorum_caches_attempts_both_endpoints_after_partial_failure() {
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 500, "{}".to_string()),
            ("/previous", 200, previous_response(0x22, 0x42)),
        ]);
        let provider = provider_for(base_url);

        assert!(provider.refresh_quorum_caches().await.is_err());
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x22; 32], 1)
                .expect("previous quorum must be cached"),
            [0x42; 48]
        );
        server.join().expect("mock quorum server must finish");

        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 200, current_response(0x33, 0x43)),
            ("/previous", 500, "{}".to_string()),
        ]);
        let provider = provider_for(base_url);

        assert!(provider.refresh_quorum_caches().await.is_err());
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x33; 32], 1)
                .expect("current quorum must be cached"),
            [0x43; 48]
        );
        server.join().expect("mock quorum server must finish");
    }

    #[tokio::test]
    async fn refresh_quorum_caches_preserves_cached_keys_when_both_endpoints_fail() {
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 500, "{}".to_string()),
            ("/previous", 500, "{}".to_string()),
        ]);
        let provider = provider_for(base_url);
        provider.current_quorums_cache.lock().unwrap().put(
            [0x44; 32],
            QuorumData {
                quorum_hash: hex::encode([0x44; 32]),
                key: hex::encode([0x54; 48]),
                height: 1,
                valid_members_count: 3,
            },
        );

        assert!(provider.refresh_quorum_caches().await.is_err());
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x44; 32], 1)
                .expect("known cached quorum must remain usable"),
            [0x54; 48]
        );
        assert!(matches!(
            provider
                .get_quorum_public_key(1, [0x45; 32], 1)
                .expect_err("unknown quorum must remain rejected"),
            ContextProviderError::InvalidQuorum(_)
        ));
        server.join().expect("mock quorum server must finish");
    }

    #[tokio::test]
    async fn unsuccessful_quorum_response_does_not_populate_cache() {
        let failed_current = serde_json::json!({
            "success": false,
            "data": [{
                "quorum_hash": hex::encode([0x55; 32]),
                "key": hex::encode([0x65; 48]),
                "height": 1,
                "valid_members_count": 3
            }]
        })
        .to_string();
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 200, failed_current),
            ("/previous", 200, empty_previous_response()),
        ]);
        let provider = provider_for(base_url);

        assert!(provider.refresh_quorum_caches().await.is_err());
        assert!(matches!(
            provider
                .get_quorum_public_key(1, [0x55; 32], 1)
                .expect_err("unsuccessful response must not populate cache"),
            ContextProviderError::InvalidQuorum(_)
        ));
        server.join().expect("mock quorum server must finish");

        let failed_previous = serde_json::json!({
            "success": false,
            "data": {
                "height": 1,
                "quorums": [{
                    "quorum_hash": hex::encode([0x56; 32]),
                    "key": hex::encode([0x76; 48]),
                    "height": 1,
                    "valid_members_count": 3
                }]
            }
        })
        .to_string();
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 200, empty_current_response()),
            ("/previous", 200, failed_previous),
        ]);
        let provider = provider_for(base_url);
        provider.previous_quorums_cache.lock().unwrap().put(
            [0x56; 32],
            QuorumData {
                quorum_hash: hex::encode([0x56; 32]),
                key: hex::encode([0x66; 48]),
                height: 1,
                valid_members_count: 3,
            },
        );

        assert!(provider.refresh_quorum_caches().await.is_err());
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x56; 32], 1)
                .expect("unsuccessful response must not overwrite cached quorum"),
            [0x66; 48]
        );
        server.join().expect("mock quorum server must finish");
    }

    #[tokio::test]
    async fn successful_empty_refresh_preserves_cached_keys() {
        let (base_url, server) = spawn_http_responses(vec![
            ("/quorums", 200, empty_current_response()),
            ("/previous", 200, empty_previous_response()),
        ]);
        let provider = provider_for(base_url);
        provider.current_quorums_cache.lock().unwrap().put(
            [0x57; 32],
            QuorumData {
                quorum_hash: hex::encode([0x57; 32]),
                key: hex::encode([0x67; 48]),
                height: 1,
                valid_members_count: 3,
            },
        );

        provider
            .refresh_quorum_caches()
            .await
            .expect("successful empty responses are valid refreshes");
        assert_eq!(
            provider
                .get_quorum_public_key(1, [0x57; 32], 1)
                .expect("empty refresh must not clear cached quorum"),
            [0x67; 48]
        );
        server.join().expect("mock quorum server must finish");
    }

    #[test]
    fn test_get_quorum_base_url() {
        assert_eq!(
            get_quorum_base_url(Network::Mainnet, None).unwrap(),
            "https://quorums.mainnet.networks.dash.org"
        );

        assert_eq!(
            get_quorum_base_url(Network::Testnet, None).unwrap(),
            "https://quorums.testnet.networks.dash.org"
        );

        assert_eq!(
            get_quorum_base_url(Network::Devnet, Some("example")).unwrap(),
            "https://quorums.example.networks.dash.org"
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
    fn test_new_with_url_rejects_plaintext_for_production_networks() {
        // Mainnet and testnet have HTTPS deployed; reject plaintext URLs to
        // prevent silently weakening the trust root via a typo or misconfig.
        // Mixed-case scheme must also be rejected (case-insensitive match).
        for url in ["http://example.com", "HTTP://example.com"] {
            for network in [Network::Mainnet, Network::Testnet] {
                let result = TrustedHttpContextProvider::new_with_url(
                    network,
                    url.to_string(),
                    NonZeroUsize::new(10).unwrap(),
                );
                match result {
                    Ok(_) => panic!("expected {} to be rejected for {:?}", url, network),
                    Err(e) => {
                        let msg = e.to_string();
                        assert!(
                            msg.contains("must use https://"),
                            "expected HTTPS-gate error for {:?} + {}, got: {}",
                            network,
                            url,
                            msg
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_new_with_url_does_not_reject_https_on_production_networks() {
        // Positive path: an HTTPS URL must not trip the new HTTPS gate.
        // The constructor may still fail downstream (DNS, on non-wasm/non-iOS
        // builds), but if so the error must NOT be the HTTPS-gate variant.
        // Mixed-case `HTTPS://` must also be accepted by the gate.
        for url in [
            "https://example.com",
            "HTTPS://example.com",
            "https://example.com:8443/sub/path",
        ] {
            for network in [Network::Mainnet, Network::Testnet] {
                let result = TrustedHttpContextProvider::new_with_url(
                    network,
                    url.to_string(),
                    NonZeroUsize::new(10).unwrap(),
                );
                if let Err(e) = result {
                    let msg = e.to_string();
                    assert!(
                        !msg.contains("must use https://"),
                        "HTTPS gate incorrectly rejected {} for {:?}: {}",
                        url,
                        network,
                        msg
                    );
                }
            }
        }
    }

    #[test]
    fn test_new_with_url_does_not_reject_plaintext_on_devnet_or_regtest() {
        // Positive path: plaintext stays acceptable for devnet/regtest (early
        // devnets without certs, loopback sidecars). The HTTPS gate must not
        // contribute to any failure here.
        for network in [Network::Devnet, Network::Regtest] {
            let result = TrustedHttpContextProvider::new_with_url(
                network,
                "http://127.0.0.1:22444".to_string(),
                NonZeroUsize::new(10).unwrap(),
            );
            if let Err(e) = result {
                let msg = e.to_string();
                assert!(
                    !msg.contains("must use https://"),
                    "HTTPS gate incorrectly rejected http:// for {:?}: {}",
                    network,
                    msg
                );
            }
        }
    }

    #[test]
    fn test_reserved_devnet_names_rejected() {
        // Names that would alias non-devnet quorum hostnames must be rejected.
        for reserved in ["mainnet", "testnet", "devnet", "local", "regtest"] {
            assert!(
                matches!(
                    get_quorum_base_url(Network::Devnet, Some(reserved)),
                    Err(TrustedContextProviderError::InvalidDevnetName(_))
                ),
                "expected '{}' to be rejected as a reserved devnet name",
                reserved
            );
        }
        // Case-insensitive: uppercase variants must also be rejected.
        for reserved in ["Mainnet", "TESTNET", "DevNet"] {
            assert!(
                matches!(
                    get_quorum_base_url(Network::Devnet, Some(reserved)),
                    Err(TrustedContextProviderError::InvalidDevnetName(_))
                ),
                "expected '{}' to be rejected as a reserved devnet name (case-insensitive)",
                reserved
            );
        }
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
