use crate::context_provider::{WasmContext, WasmTrustedContext};
use crate::error::WasmSdkError;
use dash_sdk::dpp::version::PlatformVersion;
use dash_sdk::sdk::Uri;
use dash_sdk::{Sdk, SdkBuilder};
use rs_dapi_client::{Address, RequestSettings};
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use wasm_bindgen::prelude::wasm_bindgen;

fn parse_addresses(addresses: &'static [&str]) -> Vec<Address> {
    addresses
        .iter()
        .filter_map(|addr| {
            Uri::from_maybe_shared(addr)
                .ok()
                .and_then(|uri| Address::try_from(uri).ok())
        })
        .collect()
}
// Mainnet addresses from mnowatch.org
fn default_mainnet_addresses() -> Vec<Address> {
    parse_addresses(&[
        "https://149.28.241.190:443",
        "https://198.7.115.48:443",
        "https://134.255.182.186:443",
        "https://93.115.172.39:443",
        "https://5.189.164.253:443",
    ])
}
// Testnet addresses from https://quorums.testnet.networks.dash.org/masternodes
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
fn default_local_addresses() -> Vec<Address> {
    parse_addresses(&["https://127.0.0.1:2443"])
}

#[wasm_bindgen]
pub struct WasmSdk {
    sdk: Sdk,
    trusted_context: Option<WasmTrustedContext>,
}

// Dereference WasmSdk to Sdk so that we can use &WasmSdk everywhere where &Sdk is needed
impl Deref for WasmSdk {
    type Target = Sdk;
    fn deref(&self) -> &Self::Target {
        &self.sdk
    }
}

impl AsRef<Sdk> for WasmSdk {
    fn as_ref(&self) -> &Sdk {
        &self.sdk
    }
}

#[wasm_bindgen]
impl WasmSdk {
    pub fn version(&self) -> u32 {
        self.sdk.version().protocol_version
    }

    /// Get reference to the inner SDK for direct gRPC calls
    pub(crate) fn inner_sdk(&self) -> &Sdk {
        &self.sdk
    }

    /// Get a reference to the trusted context, if available
    pub(crate) fn trusted_context(&self) -> Option<&WasmTrustedContext> {
        self.trusted_context.as_ref()
    }
}

impl WasmSdk {
    /// Add a data contract to the context provider's cache.
    pub(crate) fn add_contract_to_context_cache(
        &self,
        contract: &dash_sdk::dpp::data_contract::DataContract,
    ) -> Result<(), crate::error::WasmSdkError> {
        if let Some(ref context) = self.trusted_context {
            context.add_known_contract(contract.clone());
        }
        Ok(())
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Forces reload of the identity nonce from Platform on the next state transition.
    #[wasm_bindgen(js_name = "refreshIdentityNonce")]
    pub async fn refresh_identity_nonce(&self, identity_id: wasm_dpp2::identifier::IdentifierWasm) {
        self.sdk.refresh_identity_nonce(&identity_id.into()).await;
    }

    /// Get a cached contract from the trusted context if available
    pub(crate) fn get_cached_contract(
        &self,
        contract_id: &dash_sdk::platform::Identifier,
    ) -> Option<std::sync::Arc<dash_sdk::platform::DataContract>> {
        self.trusted_context
            .as_ref()
            .and_then(|ctx| ctx.get_known_contract(contract_id))
    }

    /// Cache a contract in the trusted context
    pub(crate) fn cache_contract(&self, contract: dash_sdk::platform::DataContract) {
        if let Some(ref context) = self.trusted_context {
            context.add_known_contract(contract);
        }
    }

    /// Fetch a contract, checking cache first
    pub(crate) async fn get_or_fetch_contract(
        &self,
        contract_id: dash_sdk::platform::Identifier,
    ) -> Result<dash_sdk::platform::DataContract, crate::error::WasmSdkError> {
        use dash_sdk::platform::Fetch;

        if let Some(cached) = self.get_cached_contract(&contract_id) {
            return Ok((*cached).clone());
        }

        let contract = dash_sdk::platform::DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| crate::error::WasmSdkError::not_found("Data contract not found"))?;

        self.cache_contract(contract.clone());

        Ok(contract)
    }

    /// Remove a contract from the cache
    pub(crate) fn remove_cached_contract(
        &self,
        contract_id: &dash_sdk::platform::Identifier,
    ) -> bool {
        self.trusted_context
            .as_ref()
            .map(|ctx| ctx.remove_known_contract(contract_id))
            .unwrap_or(false)
    }
}

#[wasm_bindgen]
impl WasmSdk {
    /// Remove a data contract from the cache.
    /// Returns true if the contract was in the cache and was removed.
    #[wasm_bindgen(js_name = "removeCachedContract")]
    pub fn remove_cached_contract_js(
        &self,
        #[wasm_bindgen(js_name = "contractId")] contract_id: &wasm_dpp2::identifier::IdentifierWasm,
    ) -> bool {
        let id: dash_sdk::platform::Identifier = (*contract_id).into();
        self.remove_cached_contract(&id)
    }
}

#[wasm_bindgen]
pub struct WasmSdkBuilder {
    inner: SdkBuilder,
    trusted_context: Option<WasmTrustedContext>,
}

impl Deref for WasmSdkBuilder {
    type Target = SdkBuilder;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for WasmSdkBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
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
    /// * `network` - Network identifier: "mainnet", "testnet" or "local"
    #[wasm_bindgen(js_name = "withAddresses")]
    pub fn new_with_addresses(
        addresses: Vec<String>,
        network: String,
    ) -> Result<Self, WasmSdkError> {
        use dash_sdk::dpp::dashcore::Network;
        use dash_sdk::sdk::Uri;

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

        let network = match network.to_lowercase().as_str() {
            "mainnet" => Network::Mainnet,
            "testnet" => Network::Testnet,
            "local" => Network::Regtest,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid network '{}'. Expected: mainnet, testnet or local",
                    network
                )));
            }
        };

        let address_list = dash_sdk::sdk::AddressList::from_iter(parsed_addresses);
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(network)
            .with_context_provider(WasmContext {});

        Ok(Self {
            inner: sdk_builder,
            trusted_context: None,
        })
    }

    #[wasm_bindgen(js_name = "mainnet")]
    pub fn new_mainnet() -> Self {
        let address_list = dash_sdk::sdk::AddressList::from_iter(default_mainnet_addresses());
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Mainnet)
            .with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
        }
    }

    #[wasm_bindgen(js_name = "testnet")]
    pub fn new_testnet() -> Self {
        let address_list = dash_sdk::sdk::AddressList::from_iter(default_testnet_addresses());
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Testnet)
            .with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
        }
    }

    /// Create a new SdkBuilder preconfigured for a local network using default dashmate gateway.
    #[wasm_bindgen(js_name = "local")]
    pub fn new_local() -> Self {
        let address_list = dash_sdk::sdk::AddressList::from_iter(default_local_addresses());
        let sdk_builder = SdkBuilder::new(address_list)
            .with_network(dash_sdk::dpp::dashcore::Network::Regtest)
            .with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
        }
    }

    /// Attach a pre-fetched trusted context to this builder.
    ///
    /// The context provides quorum keys for proof verification and
    /// discovered masternode addresses for network connectivity.
    /// If the context has discovered addresses, they replace the
    /// builder's current address list.
    ///
    /// # Example
    /// ```javascript
    /// const context = await WasmTrustedContext.prefetchTestnet();
    /// const builder = WasmSdkBuilder.testnet().withTrustedContext(context);
    /// const sdk = builder.build();
    /// ```
    #[wasm_bindgen(js_name = "withTrustedContext")]
    pub fn with_trusted_context(self, context: &WasmTrustedContext) -> Self {
        let discovered = context.discovered_addresses();

        // Replace address list with discovered addresses if available
        let inner = if !discovered.is_empty() {
            let address_list = dash_sdk::sdk::AddressList::from_iter(discovered.to_vec());
            self.inner
                .with_address_list(address_list)
                .with_context_provider(context.clone())
        } else {
            self.inner.with_context_provider(context.clone())
        };

        Self {
            inner,
            trusted_context: Some(context.clone()),
        }
    }

    pub fn build(self) -> Result<WasmSdk, WasmSdkError> {
        let sdk = self.inner.build().map_err(WasmSdkError::from)?;
        Ok(WasmSdk {
            sdk,
            trusted_context: self.trusted_context,
        })
    }

    #[wasm_bindgen(js_name = "withContextProvider")]
    pub fn with_context_provider(
        self,
        #[wasm_bindgen(js_name = "contextProvider")] context_provider: WasmContext,
    ) -> Self {
        Self {
            inner: self.inner.with_context_provider(context_provider),
            trusted_context: None,
        }
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

        Ok(Self {
            inner: self.inner.with_version(version),
            trusted_context: self.trusted_context,
        })
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

        Self {
            inner: self.inner.with_settings(settings),
            trusted_context: self.trusted_context,
        }
    }

    #[wasm_bindgen(js_name = "withProofs")]
    pub fn with_proofs(
        self,
        #[wasm_bindgen(js_name = "enableProofs")] enable_proofs: bool,
    ) -> Self {
        Self {
            inner: self.inner.with_proofs(enable_proofs),
            trusted_context: self.trusted_context,
        }
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
