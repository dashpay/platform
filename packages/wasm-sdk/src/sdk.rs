use crate::context_provider::WasmContext;
use crate::error::WasmSdkError;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::{Sdk, SdkBuilder};
use once_cell::sync::Lazy;
use rs_dapi_client::{Address, RequestSettings};
use dash_sdk::sdk::Uri;
use std::ops::{Deref, DerefMut};
use std::sync::Mutex;
use std::time::Duration;
use wasm_bindgen::prelude::wasm_bindgen;
pub(crate) static MAINNET_TRUSTED_CONTEXT: Lazy<
    Mutex<Option<crate::context_provider::WasmTrustedContext>>,
> = Lazy::new(|| Mutex::new(None));
pub(crate) static TESTNET_TRUSTED_CONTEXT: Lazy<
    Mutex<Option<crate::context_provider::WasmTrustedContext>>,
> = Lazy::new(|| Mutex::new(None));
static MAINNET_DISCOVERED_ADDRESSES: Lazy<Mutex<Option<Vec<Address>>>> =
    Lazy::new(|| Mutex::new(None));
static TESTNET_DISCOVERED_ADDRESSES: Lazy<Mutex<Option<Vec<Address>>>> =
    Lazy::new(|| Mutex::new(None));

fn parse_addresses(addresses: &[&str]) -> Vec<Address> {
    addresses
        .iter()
        .filter_map(|addr| {
            Uri::from_maybe_shared(addr.to_string())
                .ok()
                .and_then(|uri| Address::try_from(uri).ok())
        })
        .collect()
}

fn default_mainnet_addresses() -> Vec<Address> {
    // Trimmed seed list to keep bundle size small; used only if no prefetched cache is available.
    parse_addresses(&[
        "https://149.28.241.190:443",
        "https://198.7.115.48:443",
        "https://134.255.182.186:443",
        "https://93.115.172.39:443",
        "https://5.189.164.253:443",
    ])
}

fn default_testnet_addresses() -> Vec<Address> {
    parse_addresses(&[
        "https://52.12.176.90:1443",
        "https://35.82.197.197:1443",
        "https://44.240.98.102:1443",
        "https://52.34.144.50:1443",
        "https://44.239.39.153:1443",
        "https://34.214.48.68:1443",
        "https://54.149.33.167:1443",
        "https://52.24.124.162:1443",
    ])
}

async fn fetch_and_cache_addresses(
    trusted_context: &crate::context_provider::WasmTrustedContext,
    cache: &Lazy<Mutex<Option<Vec<Address>>>>,
) -> Result<(), WasmSdkError> {
    let address_list = trusted_context
        .fetch_masternode_addresses()
        .await
        .map_err(|e| WasmSdkError::generic(format!("Failed to fetch masternodes: {}", e)))?;
    let addresses: Vec<Address> = address_list
        .into_iter()
        .map(|(addr, _status)| addr)
        .collect();
    *cache.lock().unwrap() = Some(addresses);
    Ok(())
}

#[wasm_bindgen]
pub struct WasmSdk(Sdk);
// Dereference JsSdk to Sdk so that we can use &JsSdk everywhere where &sdk is needed
impl std::ops::Deref for WasmSdk {
    type Target = Sdk;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Sdk> for WasmSdk {
    fn as_ref(&self) -> &Sdk {
        &self.0
    }
}

impl From<Sdk> for WasmSdk {
    fn from(sdk: Sdk) -> Self {
        WasmSdk(sdk)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    pub fn version(&self) -> u32 {
        self.0.version().protocol_version
    }

    /// Get reference to the inner SDK for direct gRPC calls
    pub(crate) fn inner_sdk(&self) -> &Sdk {
        &self.0
    }

    /// Get the network this SDK is configured for
    pub(crate) fn network(&self) -> dash_sdk::dpp::dashcore::Network {
        self.0.network
    }
}

impl WasmSdk {
    /// Clone the inner Sdk (not exposed to WASM)
    pub(crate) fn inner_clone(&self) -> Sdk {
        self.0.clone()
    }
}

#[wasm_bindgen]
impl WasmSdk {
    #[wasm_bindgen(js_name = "prefetchTrustedQuorumsMainnet")]
    pub async fn prefetch_trusted_quorums_mainnet() -> Result<(), WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        let trusted_context = WasmTrustedContext::new_mainnet()
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        trusted_context
            .prefetch_quorums()
            .await
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        fetch_and_cache_addresses(&trusted_context, &MAINNET_DISCOVERED_ADDRESSES).await?;

        // Store the context for later use
        *MAINNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

        Ok(())
    }

    #[wasm_bindgen(js_name = "prefetchTrustedQuorumsTestnet")]
    pub async fn prefetch_trusted_quorums_testnet() -> Result<(), WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        let trusted_context = WasmTrustedContext::new_testnet()
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        trusted_context
            .prefetch_quorums()
            .await
            .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))?;

        fetch_and_cache_addresses(&trusted_context, &TESTNET_DISCOVERED_ADDRESSES).await?;

        // Store the context for later use
        *TESTNET_TRUSTED_CONTEXT.lock().unwrap() = Some(trusted_context);

        Ok(())
    }
}

#[wasm_bindgen]
pub struct WasmSdkBuilder(SdkBuilder);

impl Deref for WasmSdkBuilder {
    type Target = SdkBuilder;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WasmSdkBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[wasm_bindgen]
impl WasmSdkBuilder {
    /// Get the latest platform version number
    #[wasm_bindgen(js_name = "getLatestVersionNumber")]
    pub fn get_latest_version_number() -> u32 {
        PlatformVersion::latest().protocol_version
    }

    /// Create a new SdkBuilder with specific addresses and network.
    ///
    /// # Arguments
    /// * `addresses` - Array of HTTPS URLs (e.g., ["https://127.0.0.1:1443"])
    /// * `network` - Network identifier: "mainnet" or "testnet"
    ///
    /// # Example
    /// ```javascript
    /// const builder = WasmSdkBuilder.withAddresses(['https://127.0.0.1:1443'], 'testnet');
    /// const sdk = builder.build();
    /// ```
    #[wasm_bindgen(js_name = "withAddresses")]
    pub fn new_with_addresses(
        addresses: Vec<String>,
        network: String,
    ) -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;
        use dash_sdk::dpp::dashcore::Network;
        use dash_sdk::sdk::Uri;

        // Parse and validate addresses
        if addresses.is_empty() {
            return Err(WasmSdkError::invalid_argument(
                "Addresses must be a non-empty array",
            ));
        }
        let parsed_addresses: Result<Vec<Address>, _> = addresses
            .into_iter()
            .map(|addr| {
                addr.parse::<Uri>()
                    .map_err(|e| format!("Invalid URI '{}': {}", addr, e))
                    .and_then(|uri| {
                        Address::try_from(uri).map_err(|e| format!("Invalid address: {}", e))
                    })
            })
            .collect();

        let parsed_addresses = parsed_addresses.map_err(WasmSdkError::invalid_argument)?;

        // Parse network - only mainnet and testnet are supported
        let network = match network.to_lowercase().as_str() {
            "mainnet" => Network::Dash,
            "testnet" => Network::Testnet,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid network '{}'. Expected: mainnet or testnet",
                    network
                )));
            }
        };

        // Use the cached trusted context if available for the network, otherwise create a new one
        let trusted_context = match network {
            Network::Dash => {
                let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
                guard.clone()
            }
            .map(Ok)
            .unwrap_or_else(|| {
                WasmTrustedContext::new_mainnet()
                    .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
            })?,
            Network::Testnet => {
                let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
                guard.clone()
            }
            .map(Ok)
            .unwrap_or_else(|| {
                WasmTrustedContext::new_testnet()
                    .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
            })?,
            // Network was already validated above
            _ => unreachable!("Network already validated to mainnet or testnet"),
        };

        let address_list = dash_sdk::sdk::AddressList::from_iter(parsed_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(network)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    #[wasm_bindgen(js_name = "mainnet")]
    pub fn new_mainnet() -> Self {
        let mainnet_addresses = MAINNET_DISCOVERED_ADDRESSES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(default_mainnet_addresses);

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    #[wasm_bindgen(js_name = "mainnetTrusted")]
    pub fn new_mainnet_trusted() -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = MAINNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_mainnet()
                .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
        })?;

        let mainnet_addresses = MAINNET_DISCOVERED_ADDRESSES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(default_mainnet_addresses);

        let address_list = dash_sdk::sdk::AddressList::from_iter(mainnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Dash)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    #[wasm_bindgen(js_name = "testnet")]
    pub fn new_testnet() -> Self {
        let testnet_addresses = TESTNET_DISCOVERED_ADDRESSES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(default_testnet_addresses);

        let address_list = dash_sdk::sdk::AddressList::from_iter(testnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Testnet)
            .with_context_provider(WasmContext {});

        Self(sdk_builder)
    }

    #[wasm_bindgen(js_name = "testnetTrusted")]
    pub fn new_testnet_trusted() -> Result<Self, WasmSdkError> {
        use crate::context_provider::WasmTrustedContext;

        // Use the cached context if available, otherwise create a new one
        let trusted_context = {
            let guard = TESTNET_TRUSTED_CONTEXT.lock().unwrap();
            guard.clone()
        }
        .map(Ok)
        .unwrap_or_else(|| {
            WasmTrustedContext::new_testnet()
                .map_err(|e| WasmSdkError::from(dash_sdk::Error::from(e)))
        })?;

        let testnet_addresses = TESTNET_DISCOVERED_ADDRESSES
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(default_testnet_addresses);

        let address_list = dash_sdk::sdk::AddressList::from_iter(testnet_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Testnet)
            .with_context_provider(trusted_context);

        Ok(Self(sdk_builder))
    }

    pub fn build(self) -> Result<WasmSdk, WasmSdkError> {
        self.0.build().map(WasmSdk).map_err(WasmSdkError::from)
    }

    #[wasm_bindgen(js_name = "withContextProvider")]
    pub fn with_context_provider(
        self,
        #[wasm_bindgen(js_name = "contextProvider")] context_provider: WasmContext,
    ) -> Self {
        WasmSdkBuilder(self.0.with_context_provider(context_provider))
    }

    /// Configure platform version to use.
    ///
    /// Available versions:
    /// - 1: Platform version 1
    /// - 2: Platform version 2
    /// - ... up to latest version
    ///
    /// Defaults to latest version if not specified.
    #[wasm_bindgen(js_name = "withVersion")]
    pub fn with_version(
        self,
        #[wasm_bindgen(js_name = "versionNumber")] version_number: u32,
    ) -> Result<Self, WasmSdkError> {
        let version = PlatformVersion::get(version_number).map_err(|e| {
            WasmSdkError::invalid_argument(format!(
                "Invalid platform version {}: {}",
                version_number, e
            ))
        })?;

        Ok(WasmSdkBuilder(self.0.with_version(version)))
    }

    /// Configure request settings for the SDK.
    ///
    /// Settings include:
    /// - connect_timeout_ms: Timeout for establishing connection (in milliseconds)
    /// - timeout_ms: Timeout for single request (in milliseconds)
    /// - retries: Number of retries in case of failed requests
    /// - ban_failed_address: Whether to ban DAPI address if node not responded or responded with error
    #[wasm_bindgen(js_name = "withSettings")]
    pub fn with_settings(
        self,
        #[wasm_bindgen(js_name = "connectTimeoutMs")] connect_timeout_ms: Option<u32>,
        #[wasm_bindgen(js_name = "timeoutMs")] timeout_ms: Option<u32>,
        retries: Option<u32>,
        #[wasm_bindgen(js_name = "banFailedAddress")] ban_failed_address: Option<bool>,
    ) -> Self {
        let mut settings = RequestSettings::default();

        if let Some(connect_timeout) = connect_timeout_ms {
            settings.connect_timeout = Some(Duration::from_millis(connect_timeout as u64));
        }

        if let Some(timeout) = timeout_ms {
            settings.timeout = Some(Duration::from_millis(timeout as u64));
        }

        if let Some(retries) = retries {
            settings.retries = Some(retries as usize);
        }

        if let Some(ban) = ban_failed_address {
            settings.ban_failed_address = Some(ban);
        }

        WasmSdkBuilder(self.0.with_settings(settings))
    }

    #[wasm_bindgen(js_name = "withProofs")]
    pub fn with_proofs(
        self,
        #[wasm_bindgen(js_name = "enableProofs")] enable_proofs: bool,
    ) -> Self {
        WasmSdkBuilder(self.0.with_proofs(enable_proofs))
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Configure tracing/logging level or filter (static, global)
    ///
    /// Accepts simple levels: "off", "error", "warn", "info", "debug", "trace"
    /// or a full EnvFilter string like: "wasm_sdk=debug,rs_dapi_client=warn"
    #[wasm_bindgen(js_name = "setLogLevel")]
    pub fn set_log_level(
        #[wasm_bindgen(js_name = "levelOrFilter")] level_or_filter: &str,
    ) -> Result<(), WasmSdkError> {
        crate::logging::set_log_level(level_or_filter)
    }
}

#[wasm_bindgen]
impl WasmSdkBuilder {
    /// Configure tracing/logging via the builder
    /// Returns a new builder with logging configured
    #[wasm_bindgen(js_name = "withLogs")]
    pub fn with_logs(
        self,
        #[wasm_bindgen(js_name = "levelOrFilter")] level_or_filter: &str,
    ) -> Result<Self, WasmSdkError> {
        crate::logging::set_log_level(level_or_filter)?;
        Ok(self)
    }
}
