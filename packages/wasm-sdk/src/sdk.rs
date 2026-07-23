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

    /// Fetch a proved contract and replace its trusted-context cache entry.
    pub(crate) async fn refresh_contract(
        &self,
        contract_id: dash_sdk::platform::Identifier,
    ) -> Result<dash_sdk::platform::DataContract, crate::error::WasmSdkError> {
        use dash_sdk::platform::Fetch;

        let contract = dash_sdk::platform::DataContract::fetch(self.as_ref(), contract_id)
            .await?
            .ok_or_else(|| crate::error::WasmSdkError::not_found("Data contract not found"))?;

        self.cache_contract(contract.clone());

        Ok(contract)
    }

    /// Fetch a contract, checking cache first
    pub(crate) async fn get_or_fetch_contract(
        &self,
        contract_id: dash_sdk::platform::Identifier,
    ) -> Result<dash_sdk::platform::DataContract, crate::error::WasmSdkError> {
        if let Some(cached) = self.get_cached_contract(&contract_id) {
            return Ok((*cached).clone());
        }

        self.refresh_contract(contract_id).await
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
    /// True when the caller explicitly supplied an address list via
    /// `withAddresses(...)`. Preset constructors (mainnet/testnet/local/
    /// newDevnet) leave this `false` so that `withTrustedContext` is free
    /// to substitute its discovered masternode addresses. When `true`,
    /// `withTrustedContext` keeps the user-provided addresses and only
    /// attaches the context for proof verification.
    has_user_addresses: bool,
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
    /// * `network` - Network identifier: "mainnet", "testnet", "devnet", or "local"
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
            "devnet" => Network::Devnet,
            "local" => Network::Regtest,
            _ => {
                return Err(WasmSdkError::invalid_argument(format!(
                    "Invalid network '{}'. Expected: mainnet, testnet, devnet, or local",
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
            has_user_addresses: true,
        })
    }

    #[wasm_bindgen(js_name = "mainnet")]
    pub fn new_mainnet() -> Self {
        let sdk_builder = SdkBuilder::new_mainnet().with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
            has_user_addresses: false,
        }
    }

    #[wasm_bindgen(js_name = "testnet")]
    pub fn new_testnet() -> Self {
        let sdk_builder = SdkBuilder::new_testnet().with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
            has_user_addresses: false,
        }
    }

    /// Create a new SdkBuilder preconfigured for a devnet.
    ///
    /// Devnets have no built-in default address list. The returned builder
    /// is expected to be paired with either explicit addresses (via the
    /// `withAddresses` variant) or a `WasmTrustedContext` from
    /// `WasmTrustedContext.prefetchDevnet(name)`, whose discovered addresses
    /// will be substituted via `withTrustedContext`.
    #[wasm_bindgen(js_name = "newDevnet")]
    pub fn new_devnet() -> Self {
        let sdk_builder = SdkBuilder::new(dash_sdk::sdk::AddressList::default())
            .with_network(dash_sdk::dpp::dashcore::Network::Devnet)
            .with_context_provider(WasmContext {});

        Self {
            inner: sdk_builder,
            trusted_context: None,
            has_user_addresses: false,
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
            has_user_addresses: false,
        }
    }

    /// Attach a pre-fetched trusted context to this builder.
    ///
    /// The context provides quorum keys for proof verification and
    /// discovered masternode addresses for network connectivity.
    ///
    /// Address-list behavior:
    /// - If the builder was created via a preset (`mainnet`, `testnet`,
    ///   `local`, `newDevnet`) and the context has discovered addresses,
    ///   those discovered addresses replace the preset's default list.
    /// - If the builder was created via `withAddresses(...)`, the
    ///   user-provided addresses are preserved and discovered addresses
    ///   from the context are ignored. The context is still attached for
    ///   proof verification.
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

        // Substitute discovered addresses only when the caller did not
        // explicitly supply their own via `withAddresses(...)`.
        let inner = if !self.has_user_addresses && !discovered.is_empty() {
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
            ..self
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
            ..self
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
            ..self
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
            ..self
        }
    }

    #[wasm_bindgen(js_name = "withProofs")]
    pub fn with_proofs(
        self,
        #[wasm_bindgen(js_name = "enableProofs")] enable_proofs: bool,
    ) -> Self {
        Self {
            inner: self.inner.with_proofs(enable_proofs),
            ..self
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

#[cfg(test)]
impl WasmSdkBuilder {
    pub(crate) fn has_user_addresses(&self) -> bool {
        self.has_user_addresses
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dash_sdk::dpp::data_contract::accessors::v0::{
        DataContractV0Getters, DataContractV0Setters,
    };
    use dash_sdk::dpp::prelude::DataContract;
    use dash_sdk::dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dash_sdk::dpp::version::PlatformVersion;
    use dash_sdk::sdk::Uri;
    use rs_dapi_client::Address;

    const USER_ADDR_1: &str = "https://203.0.113.10:1443";
    const USER_ADDR_2: &str = "https://203.0.113.11:1443";
    const DISCOVERED_ADDR_1: &str = "https://198.51.100.20:1443";
    const DISCOVERED_ADDR_2: &str = "https://198.51.100.21:1443";

    fn parse(addr: &'static str) -> Address {
        Address::try_from(Uri::from_maybe_shared(addr).unwrap()).unwrap()
    }

    fn user_builder() -> WasmSdkBuilder {
        WasmSdkBuilder::new_with_addresses(
            vec![USER_ADDR_1.to_string(), USER_ADDR_2.to_string()],
            "testnet".to_string(),
        )
        .expect("withAddresses should accept valid testnet addresses")
    }

    fn sdk_addresses(sdk: &WasmSdk) -> Vec<String> {
        sdk.address_list()
            .ban_info()
            .into_iter()
            .map(|info| info.uri.to_string())
            .collect()
    }

    fn custom_contract(id_byte: u8, version: u32) -> DataContract {
        let mut contract =
            load_system_data_contract(SystemDataContract::DPNS, PlatformVersion::latest())
                .expect("DPNS contract fixture should load");
        contract.set_id(dash_sdk::platform::Identifier::new([id_byte; 32]));
        contract.set_version(version);
        contract
    }

    #[test]
    fn with_addresses_sets_user_addresses_flag() {
        let b = user_builder();
        assert!(b.has_user_addresses());
    }

    #[test]
    fn preset_constructors_do_not_set_user_addresses_flag() {
        assert!(!WasmSdkBuilder::new_mainnet().has_user_addresses());
        assert!(!WasmSdkBuilder::new_testnet().has_user_addresses());
        assert!(!WasmSdkBuilder::new_local().has_user_addresses());
        assert!(!WasmSdkBuilder::new_devnet().has_user_addresses());
    }

    #[test]
    fn chain_methods_preserve_user_addresses_flag() {
        let b = user_builder()
            .with_proofs(true)
            .with_settings(Some(1000), Some(2000), Some(1), Some(false))
            .with_version(1)
            .expect("withVersion(1) is valid")
            .with_context_provider(WasmContext {})
            .with_logs("off")
            .expect("withLogs(\"off\") is valid");
        assert!(b.has_user_addresses());

        let preset = WasmSdkBuilder::new_testnet()
            .with_proofs(true)
            .with_settings(None, None, None, None);
        assert!(!preset.has_user_addresses());
    }

    /// Repro of the bug behind the fix: a user calling
    /// `withAddresses([...]).withTrustedContext(ctx)` must keep their
    /// explicit addresses, not get them silently replaced by discovered
    /// masternodes from the trusted context.
    #[test]
    fn with_trusted_context_preserves_user_addresses() {
        let ctx = WasmTrustedContext::for_testing(vec![
            parse(DISCOVERED_ADDR_1),
            parse(DISCOVERED_ADDR_2),
        ]);

        let sdk = user_builder()
            .with_trusted_context(&ctx)
            .build()
            .expect("build should succeed with explicit addresses + trusted context");

        let addrs = sdk_addresses(&sdk);

        assert_eq!(
            addrs.len(),
            2,
            "user addresses must be preserved (got {:?})",
            addrs
        );
        assert!(
            addrs.iter().any(|a| a.contains("203.0.113.10")),
            "missing first user address in {:?}",
            addrs
        );
        assert!(
            addrs.iter().any(|a| a.contains("203.0.113.11")),
            "missing second user address in {:?}",
            addrs
        );
        assert!(
            !addrs.iter().any(|a| a.contains("198.51.100")),
            "discovered address must not leak in: {:?}",
            addrs
        );
    }

    /// Counterpart: a preset builder + trusted context with discovered
    /// addresses must adopt the discovered list. This guards against an
    /// over-correction that would always preserve the preset defaults.
    #[test]
    fn with_trusted_context_replaces_preset_addresses() {
        let ctx = WasmTrustedContext::for_testing(vec![
            parse(DISCOVERED_ADDR_1),
            parse(DISCOVERED_ADDR_2),
        ]);

        let sdk = WasmSdkBuilder::new_testnet()
            .with_trusted_context(&ctx)
            .build()
            .expect("build should succeed for preset + trusted context");

        let addrs = sdk_addresses(&sdk);

        assert_eq!(
            addrs.len(),
            2,
            "preset addresses should be replaced by the two discovered ones (got {:?})",
            addrs
        );
        assert!(
            addrs.iter().all(|a| a.contains("198.51.100")),
            "preset addresses must be replaced by discovered ones: {:?}",
            addrs
        );
    }

    /// A trusted context with no discovered addresses should never wipe
    /// the existing address list, regardless of where it came from.
    #[test]
    fn with_trusted_context_no_discovered_keeps_existing_addresses() {
        let ctx = WasmTrustedContext::for_testing(vec![]);
        let sdk = user_builder()
            .with_trusted_context(&ctx)
            .build()
            .expect("build should succeed");
        assert_eq!(sdk.address_list().ban_info().len(), 2);
    }

    /// Flag must survive chaining *through* withTrustedContext as well —
    /// later builder steps must still see has_user_addresses=true.
    #[test]
    fn with_trusted_context_propagates_user_addresses_flag() {
        let ctx = WasmTrustedContext::for_testing(vec![parse(DISCOVERED_ADDR_1)]);
        let b = user_builder().with_trusted_context(&ctx);
        assert!(b.has_user_addresses());
    }

    #[tokio::test]
    async fn refresh_contract_caches_a_missing_custom_contract() {
        let expected = custom_contract(0x44, 1);
        let contract_id = expected.id();
        let mut inner_sdk = Sdk::new_mock();
        inner_sdk
            .mock()
            .expect_fetch(contract_id, Some(expected.clone()))
            .await
            .expect("mock contract response should be configured");

        let sdk = WasmSdk {
            sdk: inner_sdk,
            trusted_context: Some(WasmTrustedContext::for_testing(vec![])),
        };
        assert!(sdk.get_cached_contract(&contract_id).is_none());

        let refreshed = sdk
            .refresh_contract(contract_id)
            .await
            .expect("proved contract fetch should succeed");

        assert_eq!(refreshed, expected);
        assert_eq!(
            sdk.get_cached_contract(&contract_id)
                .expect("refreshed contract should be cached")
                .as_ref(),
            &expected,
        );
    }

    #[tokio::test]
    async fn refresh_contract_replaces_a_stale_cached_version() {
        let stale = custom_contract(0x55, 1);
        let expected = custom_contract(0x55, 2);
        let contract_id = expected.id();
        let context = WasmTrustedContext::for_testing(vec![]);
        context.add_known_contract(stale);

        let mut inner_sdk = Sdk::new_mock();
        inner_sdk
            .mock()
            .expect_fetch(contract_id, Some(expected.clone()))
            .await
            .expect("mock contract response should be configured");

        let sdk = WasmSdk {
            sdk: inner_sdk,
            trusted_context: Some(context),
        };
        let refreshed = sdk
            .refresh_contract(contract_id)
            .await
            .expect("proved contract refresh should succeed");

        assert_eq!(refreshed.version(), 2);
        assert_eq!(
            sdk.get_cached_contract(&contract_id)
                .expect("latest contract should replace stale cache")
                .version(),
            2,
        );
    }

    #[tokio::test]
    async fn refresh_contract_propagates_proved_absence() {
        let contract_id = dash_sdk::platform::Identifier::new([0x66; 32]);
        let mut inner_sdk = Sdk::new_mock();
        inner_sdk
            .mock()
            .expect_fetch(contract_id, None as Option<DataContract>)
            .await
            .expect("mock absence response should be configured");

        let sdk = WasmSdk {
            sdk: inner_sdk,
            trusted_context: Some(WasmTrustedContext::for_testing(vec![])),
        };

        assert!(sdk.refresh_contract(contract_id).await.is_err());
        assert!(sdk.get_cached_contract(&contract_id).is_none());
    }

    #[tokio::test]
    async fn refresh_contract_propagates_fetch_errors() {
        let contract_id = dash_sdk::platform::Identifier::new([0x77; 32]);
        let sdk = WasmSdk {
            sdk: Sdk::new_mock(),
            trusted_context: Some(WasmTrustedContext::for_testing(vec![])),
        };

        assert!(sdk.refresh_contract(contract_id).await.is_err());
        assert!(sdk.get_cached_contract(&contract_id).is_none());
    }
}
