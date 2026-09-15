use std::sync::Arc;

use dash_sdk::dpp::dashcore::Network;
use dash_sdk::platform::ContextProvider;
use dash_sdk::{
    dpp::{data_contract::TokenConfiguration, prelude::CoreBlockHeight, version::PlatformVersion},
    error::ContextProviderError,
    platform::{DataContract, Identifier},
};
use rs_sdk_trusted_context_provider::TrustedHttpContextProvider;
use wasm_bindgen::prelude::wasm_bindgen;

use crate::error::WasmSdkError;

#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmContext {}

/// A wrapper for TrustedHttpContextProvider that works in WASM.
///
/// Holds pre-fetched quorum keys for proof verification and, unless discovery
/// was skipped, the masternode addresses the quorum service advertises. Create
/// one via the async `prefetchMainnet()`, `prefetchTestnet()`,
/// `prefetchDevnet()`, or `prefetchLocal()` factory methods, then pass it to a
/// builder via `withTrustedContext()`.
///
/// Every factory takes an optional trailing `discoverAddresses` flag. The
/// current and previous quorum lists are always fetched, concurrently; the
/// masternode list is fetched alongside them unless the flag is `false`. A
/// builder created with `WasmSdkBuilder.withAddresses(...)` ignores discovered
/// addresses, so callers with explicit addresses should pass `false` and save
/// the round trip.
#[wasm_bindgen]
#[derive(Clone)]
pub struct WasmTrustedContext {
    inner: std::sync::Arc<TrustedHttpContextProvider>,
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
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchMainnet")]
    pub async fn prefetch_mainnet(
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Mainnet, None, None, discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for mainnet using a
    /// fully-specified quorum base URL (useful for testing against a staging
    /// or self-hosted quorums endpoint).
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchMainnetWithUrl")]
    pub async fn prefetch_mainnet_with_url(
        base_url: String,
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Mainnet, None, Some(base_url), discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for testnet.
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.testnet().withTrustedContext(context)`.
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchTestnet")]
    pub async fn prefetch_testnet(
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Testnet, None, None, discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for testnet using a
    /// fully-specified quorum base URL (useful for testing against a staging
    /// or self-hosted quorums endpoint).
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchTestnetWithUrl")]
    pub async fn prefetch_testnet_with_url(
        base_url: String,
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Testnet, None, Some(base_url), discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for a devnet.
    ///
    /// `devnet_name` is the short name of the devnet (e.g. `"paloma"`). The
    /// quorum base URL is derived as `https://quorums.<devnet_name>.networks.dash.org`.
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.newDevnet().withTrustedContext(context)`.
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchDevnet")]
    pub async fn prefetch_devnet(
        devnet_name: String,
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Devnet, Some(devnet_name), None, discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for a devnet using a
    /// fully-specified quorum base URL.
    ///
    /// Use this when the default
    /// `https://quorums.<devnet_name>.networks.dash.org` URL produced by
    /// `prefetchDevnet` is not yet deployed for a devnet, or when pointing
    /// at a non-standard quorums endpoint.
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchDevnetWithUrl")]
    pub async fn prefetch_devnet_with_url(
        base_url: String,
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Devnet, None, Some(base_url), discover_addresses).await
    }

    /// Pre-fetch quorum keys and masternode addresses for a local network.
    ///
    /// Uses the default local quorum sidecar URL (`http://127.0.0.1:22444`).
    ///
    /// Returns a ready-to-use `WasmTrustedContext` that can be passed to
    /// `WasmSdkBuilder.local().withTrustedContext(context)`.
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchLocal")]
    pub async fn prefetch_local(
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_local_with_url("http://127.0.0.1:22444".to_string(), discover_addresses)
            .await
    }

    /// Pre-fetch quorum keys and masternode addresses for a local network
    /// using a custom quorum sidecar URL.
    ///
    /// Pass `discoverAddresses: false` to skip the masternode discovery request
    /// when the SDK is built with `WasmSdkBuilder.withAddresses(...)`, which
    /// ignores discovered addresses anyway. Defaults to `true`.
    #[wasm_bindgen(js_name = "prefetchLocalWithUrl")]
    pub async fn prefetch_local_with_url(
        base_url: String,
        #[wasm_bindgen(js_name = "discoverAddresses")] discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        Self::prefetch_for(Network::Regtest, None, Some(base_url), discover_addresses).await
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
    /// `discover_addresses` defaults to `true`; see the type-level docs.
    async fn prefetch_for(
        network: Network,
        devnet_name: Option<String>,
        base_url: Option<String>,
        discover_addresses: Option<bool>,
    ) -> Result<WasmTrustedContext, WasmSdkError> {
        let cache_size = std::num::NonZeroUsize::new(100).unwrap();
        let inner = match base_url {
            Some(url) => TrustedHttpContextProvider::new_with_url(network, url, cache_size),
            None => TrustedHttpContextProvider::new(network, devnet_name, cache_size),
        }
        .map_err(|e| WasmSdkError::generic(format!("Failed to create context provider: {}", e)))?
        .with_refetch_if_not_found(false);

        let inner = Arc::new(inner);

        // The quorum keys are what the context is for; the masternode list only
        // feeds `withTrustedContext` on builders without explicit addresses.
        // Everything that is fetched goes out at once, so the boot path pays a
        // single round trip instead of one per request.
        let quorums = async {
            inner
                .update_quorum_caches()
                .await
                .map_err(|e| WasmSdkError::generic(format!("Failed to prefetch quorums: {}", e)))
        };
        let discovered_addresses = if discover_addresses.unwrap_or(true) {
            let ((), addresses) = futures::try_join!(quorums, Self::fetch_addresses_from(&inner))?;
            addresses
        } else {
            quorums.await?;
            Vec::new()
        };

        Ok(WasmTrustedContext {
            inner,
            discovered_addresses,
        })
    }

    /// Fetch masternode addresses from the trusted provider and convert to `Vec<Address>`.
    async fn fetch_addresses_from(
        inner: &TrustedHttpContextProvider,
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

    /// Get the discovered addresses (for use by the builder). Empty when the
    /// context was prefetched with `discoverAddresses: false`.
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
        let inner =
            TrustedHttpContextProvider::new_with_url(Network::Regtest, base_url, cache_size)
                .expect("test URL must construct a TrustedHttpContextProvider")
                .with_refetch_if_not_found(false);

        WasmTrustedContext {
            inner: Arc::new(inner),
            discovered_addresses,
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

    fn quorums_body(hash: u8, key: u8) -> String {
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

    fn previous_body(hash: u8, key: u8) -> String {
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

    fn masternodes_body() -> String {
        serde_json::json!({
            "success": true,
            "data": [
                {
                    "address": "203.0.113.5:9999",
                    "status": "ENABLED",
                    "versionCheck": "success",
                    "platformHTTPPort": 1443
                },
                {
                    "address": "203.0.113.6:9999",
                    "status": "POSE_BANNED",
                    "versionCheck": "success",
                    "platformHTTPPort": 1443
                }
            ]
        })
        .to_string()
    }

    /// Serve one `200` response per `(path, body)`, matched by request path.
    /// With `hold_until_all_connected`, every expected connection is accepted
    /// before any is answered: a client that issued the requests one after
    /// another would never open the next one, the accept deadline would fire,
    /// and the held connection would drop, failing the client. Passing proves
    /// the requests were in flight together. A request for a path with no
    /// pending entry panics the server thread, which `join()` reports.
    fn spawn_endpoint(
        responses: Vec<(&str, String)>,
        hold_until_all_connected: bool,
    ) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock quorum endpoint");
        listener
            .set_nonblocking(true)
            .expect("make mock endpoint bounded");
        let address = listener.local_addr().expect("read mock endpoint address");
        let mut pending = responses
            .into_iter()
            .map(|(path, body)| (path.to_string(), body))
            .collect::<Vec<_>>();

        let handle = thread::spawn(move || {
            let mut held = Vec::new();
            while !pending.is_empty() {
                let mut stream = accept_before(&listener, Instant::now() + Duration::from_secs(5));
                let path = read_request_path(&stream);
                let index = pending
                    .iter()
                    .position(|(expected_path, _)| *expected_path == path)
                    .unwrap_or_else(|| panic!("unexpected quorum request path {path}"));
                let (_, body) = pending.remove(index);
                if hold_until_all_connected {
                    held.push((stream, body));
                } else {
                    write_response(&mut stream, &body);
                }
            }
            for (mut stream, body) in held {
                write_response(&mut stream, &body);
            }
        });

        (format!("http://{}", address), handle)
    }

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

    fn read_request_path(stream: &TcpStream) -> String {
        let mut reader = BufReader::new(stream.try_clone().expect("clone quorum request stream"));
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .expect("read quorum request line");
        let path = request_line
            .split_whitespace()
            .nth(1)
            .expect("quorum request line must carry a path")
            .to_string();
        loop {
            let mut header = String::new();
            reader
                .read_line(&mut header)
                .expect("read quorum request header");
            if header == "\r\n" || header.is_empty() {
                break;
            }
        }
        path
    }

    fn write_response(stream: &mut TcpStream, body: &str) {
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write quorum response");
        stream.flush().expect("flush quorum response");
    }

    #[tokio::test]
    async fn prefetch_skips_masternode_discovery_when_disabled() {
        let (base_url, server) = spawn_endpoint(
            vec![
                ("/quorums", quorums_body(0x11, 0x41)),
                ("/previous", previous_body(0x12, 0x42)),
            ],
            false,
        );

        let context =
            WasmTrustedContext::prefetch_for(Network::Regtest, None, Some(base_url), Some(false))
                .await
                .expect("quorum-only prefetch must succeed");

        assert!(context.discovered_addresses().is_empty());
        assert_eq!(
            context
                .get_quorum_public_key(1, [0x11; 32], 1)
                .expect("current quorum key must be cached"),
            [0x41; 48]
        );
        assert_eq!(
            context
                .get_quorum_public_key(1, [0x12; 32], 1)
                .expect("previous quorum key must be cached"),
            [0x42; 48]
        );
        server
            .join()
            .expect("the endpoint must see exactly the two quorum requests");
    }

    #[tokio::test]
    async fn prefetch_discovers_addresses_in_the_same_round_trip_as_quorums() {
        let (base_url, server) = spawn_endpoint(
            vec![
                ("/quorums", quorums_body(0x21, 0x51)),
                ("/previous", previous_body(0x22, 0x52)),
                ("/masternodes", masternodes_body()),
            ],
            true,
        );

        let context =
            WasmTrustedContext::prefetch_for(Network::Regtest, None, Some(base_url), None)
                .await
                .expect("prefetch with discovery must succeed");

        let discovered: Vec<(Option<&str>, Option<u16>)> = context
            .discovered_addresses()
            .iter()
            .map(|address| (address.uri().host(), address.uri().port_u16()))
            .collect();
        assert_eq!(discovered, vec![(Some("203.0.113.5"), Some(1443))]);
        assert_eq!(
            context
                .get_quorum_public_key(1, [0x21; 32], 1)
                .expect("current quorum key must be cached"),
            [0x51; 48]
        );
        server
            .join()
            .expect("all three requests must have been in flight together");
    }
}
