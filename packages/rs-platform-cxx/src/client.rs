// Copyright (c) 2026 The Dash Core developers
// Distributed under the MIT software license, see the accompanying
// file COPYING or http://www.opensource.org/licenses/mit-license.php.

//! The embedder's handle on `dash-sdk`: the runtime, the provider, the
//! `Sdk` built lazily from the endpoint set the embedder pushes (and dropped
//! while that set is empty), and the shell's own Platform-height watermark
//! and verified protocol version, which outlive any one `Sdk`.

use std::collections::HashSet;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use dash_sdk::dapi_client::transport::{ProxyEndpoint, Socks5Auth, Socks5Proxy};
use dash_sdk::dpp::dashcore::Network;
use dash_sdk::dpp::version::{PlatformVersion, LATEST_VERSION};
use dash_sdk::sdk::{min_protocol_version, Address, AddressList};
use dash_sdk::{RequestSettings, Sdk, SdkBuilder};

use crate::ffi::{self, Status};
use crate::provider::LocalContextProvider;
use crate::runtime::{RunError, Runtime};
use crate::sync::{read, write};

/// Per-request deadline. The SDK may retry across endpoints inside it and
/// spend more than `(REQUEST_TIMEOUT + CONNECT_TIMEOUT) * RETRIES` in total.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
/// Budget to open a TLS connection to one evonode.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// The same through the proxy: building a Tor circuit plus the SOCKS5
/// handshake plus TLS (Dash Core allows 20 s for each SOCKS5 reply).
const PROXIED_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Retries per logical request; each may hit another endpoint as the SDK
/// bans failing ones.
const RETRIES: usize = 2;
/// Largest gRPC response the SDK decodes (tonic's default).
const MAX_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
/// The SDK's signed-time window: a verified response whose signed time is
/// further than this from the local clock is stale or replayed.
const TIME_TOLERANCE: Duration = Duration::from_secs(10 * 60);
/// Platform blocks a verified response may trail the highest verified
/// height before the shell refuses it. The SDK's own watermark (default
/// tolerance 1, which would ban honest evonodes one block apart) is off.
pub const HEIGHT_TOLERANCE: u64 = 3;
/// The `Unavailable` reason after `shutdown`.
const SHUT_DOWN: &str = "platform client is shut down";

pub struct Client {
    network: Network,
    /// Fixed for the client's lifetime: the embedder's proxy settings do
    /// not change without a restart, so no SDK ever runs without it.
    proxy: Option<Socks5Proxy>,
    provider: Arc<LocalContextProvider>,
    runtime: Runtime,
    sdk: RwLock<Option<Sdk>>,
    /// The endpoint set the embedder last pushed; the SDK's live list is
    /// diffed against it, so ban state of retained entries is untouched.
    endpoints: RwLock<HashSet<Address>>,
    /// Highest Platform height verified so far (0 = none yet).
    last_seen_height: AtomicU64,
    /// Highest protocol version a response the shell accepted has carried
    /// (0 = none yet), known to this build or not. Kept apart from the
    /// SDK's ratchet, which runs inside proof verification before the
    /// shell's height watermark.
    verified_protocol_version: AtomicU32,
}

impl Client {
    pub fn new(cfg: &ffi::Config) -> Result<Self, String> {
        let network = match cfg.network {
            0 => Network::Mainnet,
            1 => Network::Testnet,
            2 => Network::Devnet,
            3 => Network::Regtest,
            other => return Err(format!("unknown network {other}")),
        };
        Ok(Client {
            network,
            proxy: proxy(&cfg.proxy)?,
            provider: Arc::new(LocalContextProvider::new(cfg.platform_llmq_type)),
            runtime: Runtime::new()?,
            sdk: RwLock::new(None),
            endpoints: RwLock::new(HashSet::new()),
            last_seen_height: AtomicU64::new(0),
            verified_protocol_version: AtomicU32::new(0),
        })
    }

    pub fn provider(&self) -> &Arc<LocalContextProvider> {
        &self.provider
    }

    /// Replaces the evonode endpoint set. Only `https` endpoints are
    /// accepted: the address list takes any scheme and would dial `http` in
    /// the clear. The host must be an IP address, or an onion name when a
    /// proxy is configured: nothing is ever resolved locally, and an onion
    /// name is reachable only through the proxy.
    ///
    /// Removing an address from the SDK's list only stops new requests
    /// going to it: the DAPI client keeps a pooled channel per endpoint it
    /// has talked to, and the channel holds its connection open for as long
    /// as the pool does. The pool is private to the DAPI client, so the only
    /// way to close those connections is to drop the client. Hence:
    /// - an empty set empties the shared address list, so a request still
    ///   in flight makes no further attempt, and drops the SDK, closing
    ///   every connection (`DapiClient::new` would panic on an empty list
    ///   anyway); the next non-empty set builds a new one;
    /// - a set that removes entries builds a new SDK over the same, updated
    ///   address list, so the connections to removed nodes close while the
    ///   retained entries keep their ban state; retained nodes reconnect on
    ///   their next request;
    /// - a set that only adds entries updates the list in place.
    ///
    /// The provider, the height watermark and the verified protocol version
    /// belong to the client and survive every rebuild.
    pub fn set_endpoints(&self, https_uris: &[String]) -> Result<(), String> {
        let wanted = https_uris
            .iter()
            .map(|uri| {
                let address =
                    Address::from_str(uri).map_err(|e| format!("bad evonode endpoint: {e}"))?;
                if address.uri().scheme_str() != Some("https") {
                    return Err(format!("bad evonode endpoint: {uri} is not https"));
                }
                self.check_host(uri, address.uri().host().unwrap_or_default())?;
                Ok(address)
            })
            .collect::<Result<HashSet<Address>, String>>()?;
        let mut sdk = write(&self.sdk);
        let mut endpoints = write(&self.endpoints);
        let retired = match sdk.as_ref() {
            None if wanted.is_empty() => None,
            None => {
                *sdk = Some(self.build_sdk(wanted.iter().cloned().collect())?);
                None
            }
            Some(current) if wanted.is_empty() => {
                // An in-flight request holds a clone of the SDK and picks
                // an address from this shared list for every attempt;
                // emptying it (iterating an `AddressList` takes its
                // entries) stops that request dialling the old nodes again.
                current.address_list().clone().into_iter().for_each(drop);
                sdk.take()
            }
            Some(current) => {
                let mut live = current.address_list().clone();
                for added in wanted.difference(&endpoints) {
                    live.add(added.clone());
                }
                if endpoints.is_subset(&wanted) {
                    None
                } else {
                    for gone in endpoints.difference(&wanted) {
                        live.remove(gone);
                    }
                    sdk.replace(self.build_sdk(live)?)
                }
            }
        };
        // Recorded only once the SDK follows the set. The live list is
        // shared with the current SDK, so after a failed rebuild it already
        // routes by the new set, but the removed nodes' channels are still
        // pooled; diffing the next call against the old set rebuilds then.
        *endpoints = wanted;
        drop(endpoints);
        drop(sdk);
        if let Some(retired) = retired {
            self.retire(retired);
        }
        Ok(())
    }

    fn check_host(&self, uri: &str, host: &str) -> Result<(), String> {
        let unbracketed = host
            .strip_prefix('[')
            .and_then(|host| host.strip_suffix(']'))
            .unwrap_or(host);
        if unbracketed.parse::<IpAddr>().is_ok() {
            return Ok(());
        }
        if host.ends_with(".onion") {
            return match self.proxy {
                Some(_) => Ok(()),
                None => Err(format!(
                    "bad evonode endpoint: {uri} is an onion name and no proxy is configured"
                )),
            };
        }
        Err(format!(
            "bad evonode endpoint: {uri} is neither an IP address nor an onion name"
        ))
    }

    /// A network SDK over `addresses`. Once a read has been accepted it is
    /// seeded at the verified protocol version (capped at the latest this
    /// build knows), so a rebuilt SDK does not fall back to the network
    /// floor.
    fn build_sdk(&self, addresses: AddressList) -> Result<Sdk, String> {
        let mut builder = self.configure(SdkBuilder::new(addresses));
        let verified = self.verified_protocol_version.load(Ordering::Acquire);
        if verified != 0 {
            let seed = verified.clamp(self.floor_version().protocol_version, LATEST_VERSION);
            builder = builder.with_initial_version(
                PlatformVersion::get(seed)
                    .map_err(|e| format!("unknown verified protocol version: {e}"))?,
            );
        }
        builder
            .build()
            .map_err(|e| format!("unable to build the Platform SDK: {e}"))
    }

    /// Drops `sdk`, and with it the connection pool once no in-flight
    /// request still holds a clone (at most until that request's deadline).
    /// The pool is dropped inside the runtime context: tonic's channels
    /// expect a reactor on drop.
    fn retire(&self, sdk: Sdk) {
        match self.runtime.handle() {
            Some(handle) => {
                let _entered = handle.enter();
                drop(sdk);
            }
            None => drop(sdk),
        }
    }

    /// The shell's SDK policy on any builder: proofs on, this provider,
    /// the proxy, the request budget, the signed-time window, and the SDK's
    /// own height watermark off (the shell keeps its own, see
    /// [`HEIGHT_TOLERANCE`]).
    fn configure(&self, builder: SdkBuilder) -> SdkBuilder {
        let (builder, connect_timeout) = match &self.proxy {
            Some(proxy) => (builder.with_proxy(proxy.clone()), PROXIED_CONNECT_TIMEOUT),
            None => (builder, CONNECT_TIMEOUT),
        };
        builder
            .with_network(self.network)
            .with_proofs(true)
            .with_context_provider(Arc::clone(&self.provider))
            .with_settings(RequestSettings {
                connect_timeout: Some(connect_timeout),
                timeout: Some(REQUEST_TIMEOUT),
                retries: Some(RETRIES),
                ban_failed_address: Some(true),
                max_decoding_message_size: Some(MAX_RESPONSE_BYTES),
            })
            .with_time_tolerance(Some(TIME_TOLERANCE.as_millis() as u64))
            .with_height_tolerance(None)
    }

    /// A mock-transport SDK builder under the same policy as the network
    /// SDK, for tests that replay recorded responses through this client.
    #[cfg(feature = "mocks")]
    pub fn mock_sdk_builder(&self) -> SdkBuilder {
        self.configure(SdkBuilder::new_mock())
    }

    /// Installs a ready-made SDK (tests use a mock one) in place of the
    /// lazily built network SDK.
    #[cfg(feature = "mocks")]
    pub fn set_sdk(&self, sdk: Sdk) {
        *write(&self.sdk) = Some(sdk);
    }

    /// The SDK instance, once endpoints have been pushed.
    pub fn sdk(&self) -> Result<Sdk, String> {
        read(&self.sdk)
            .clone()
            .ok_or_else(|| "no evonode endpoints".to_string())
    }

    /// Whether a request can be dispatched at all: the client is not shut
    /// down and endpoints have been pushed. `Unavailable` otherwise.
    pub fn check_ready(&self) -> Result<(), Status> {
        if self.runtime.handle().is_none() {
            return Err(Status::unavailable(SHUT_DOWN));
        }
        if read(&self.sdk).is_none() {
            return Err(Status::unavailable("no evonode endpoints"));
        }
        Ok(())
    }

    /// [`Self::check_ready`] plus the trust anchor a proved read needs: a
    /// local ChainLock height. The provider refuses every proof without one
    /// anyway; not dispatching keeps the SDK from banning honest nodes for
    /// the embedder's own missing state.
    pub fn check_ready_for_proofs(&self) -> Result<(), Status> {
        self.check_ready()?;
        if self.provider.local_core_chain_locked_height() == 0 {
            return Err(Status::unavailable(
                "no local ChainLock anchor pushed yet; proved reads are not dispatched",
            ));
        }
        Ok(())
    }

    /// The SDK's view of the protocol version: the network floor until a
    /// verified response ratchets it. The SDK ratchets inside proof
    /// verification, before the shell's height watermark, so a response the
    /// shell then refuses can move it; nothing the shell decides reads it.
    pub fn platform_version(&self) -> &'static PlatformVersion {
        match read(&self.sdk).as_ref() {
            Some(sdk) => sdk.version(),
            None => self.floor_version(),
        }
    }

    fn floor_version(&self) -> &'static PlatformVersion {
        PlatformVersion::get(min_protocol_version(self.network))
            .expect("the network floor is a known protocol version")
    }

    /// Records the protocol version of a response the shell accepted (after
    /// the height watermark). Monotonic; the signed metadata covers it, so
    /// it is trusted as far as the quorum is.
    pub fn observe_protocol_version(&self, version: u32) {
        self.verified_protocol_version
            .fetch_max(version, Ordering::AcqRel);
    }

    /// The protocol version a state transition must be built under: the
    /// highest one a response the shell accepted has shown the network to
    /// run. Before that only the network floor is known, and a transition
    /// built there can differ from what the network expects (from protocol
    /// version 14 a document id commits to the identity contract nonce; the
    /// contested prefund changed in the same version), so building is
    /// refused until a read has verified the version. The floor equals the
    /// latest known version on devnets, where nothing higher can be learned.
    /// Once the network has been seen to run a version this build does not
    /// know, building is refused as well: the reads signal
    /// `UnsupportedProtocolVersion` and the embedder must update. The
    /// version only moves up; a network upgrade is seen on the next accepted
    /// read, which the embedder makes (the nonce) right before building.
    pub fn verified_platform_version(&self) -> Result<&'static PlatformVersion, String> {
        let floor = self.floor_version();
        match self.verified_protocol_version.load(Ordering::Acquire) {
            0 if floor.protocol_version == LATEST_VERSION => Ok(floor),
            0 => Err(format!(
                "no verified read yet: the network's protocol version is unknown (the {} floor \
                 is {}, this build knows {LATEST_VERSION})",
                self.network, floor.protocol_version
            )),
            verified if verified > LATEST_VERSION => Err(format!(
                "the network runs protocol version {verified}, this build knows up to \
                 {LATEST_VERSION}; no transition can be built until it is updated"
            )),
            verified => PlatformVersion::get(verified.max(floor.protocol_version))
                .map_err(|e| format!("unknown verified protocol version: {e}")),
        }
    }

    /// Highest Platform height verified so far.
    pub fn last_seen_height(&self) -> u64 {
        self.last_seen_height.load(Ordering::Acquire)
    }

    /// Advances the watermark to `height` and reports whether a response at
    /// that height is fresh: at most [`HEIGHT_TOLERANCE`] blocks behind the
    /// highest height seen, itself included.
    pub fn observe_height(&self, height: u64) -> bool {
        let previous = self.last_seen_height.fetch_max(height, Ordering::AcqRel);
        let expected = previous.max(self.last_seen_height.load(Ordering::Acquire));
        !(expected > HEIGHT_TOLERANCE && height < expected - HEIGHT_TOLERANCE)
    }

    /// Runs one SDK operation on the runtime, blocking the calling thread.
    /// The SDK is cloned out of its lock so concurrent callers do not
    /// serialize on it; the clone shares the address list and version
    /// state.
    pub fn run<F, Fut, T>(&self, op: F) -> Result<T, ffi::Status>
    where
        F: FnOnce(Sdk) -> Fut,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        self.check_ready()?;
        let sdk = self.sdk().map_err(Status::unavailable)?;
        self.runtime.run(op(sdk)).map_err(|error| match error {
            RunError::ShutDown => Status::unavailable(SHUT_DOWN),
            RunError::Panicked(message) => Status::internal(message),
        })
    }

    /// Aborts the in-flight request, cancels the SDK and stops the runtime
    /// (bounded by the runtime's shutdown timeout). Reads afterwards return
    /// `Unavailable`. Idempotent.
    pub fn shutdown(&self) {
        // The connection pool goes before the runtime itself.
        let sdk = write(&self.sdk).take();
        if let Some(sdk) = sdk {
            sdk.shutdown();
            self.retire(sdk);
        }
        self.runtime.shutdown();
    }
}

/// The embedder's proxy setting as the DAPI client's. Anything malformed
/// is an error, so the embedder gets no client rather than one that
/// connects directly.
fn proxy(cfg: &ffi::Proxy) -> Result<Option<Socks5Proxy>, String> {
    let endpoint = match cfg.kind {
        0 if cfg.address.is_empty() => return Ok(None),
        0 => return Err("a proxy address is set without a proxy kind".to_string()),
        1 => ProxyEndpoint::Tcp(SocketAddr::from_str(&cfg.address).map_err(|_| {
            format!(
                "the proxy address {:?} is not a numeric ip:port",
                cfg.address
            )
        })?),
        #[cfg(unix)]
        2 if !cfg.address.is_empty() => ProxyEndpoint::Unix(cfg.address.clone().into()),
        #[cfg(unix)]
        2 => return Err("the proxy socket path is empty".to_string()),
        other => return Err(format!("unsupported proxy kind {other}")),
    };
    let auth = if cfg.isolate {
        Socks5Auth::RandomPerConnection
    } else {
        Socks5Auth::None
    };
    Ok(Some(Socks5Proxy { endpoint, auth }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_proxy() -> ffi::Proxy {
        ffi::Proxy {
            kind: 0,
            address: String::new(),
            isolate: false,
        }
    }

    fn config(proxy: ffi::Proxy) -> ffi::Config {
        ffi::Config {
            network: 1,
            platform_llmq_type: 106,
            proxy,
        }
    }

    fn testnet_client() -> Client {
        Client::new(&config(no_proxy())).expect("client")
    }

    fn proxied_client() -> Client {
        Client::new(&config(ffi::Proxy {
            kind: 1,
            address: "127.0.0.1:9050".to_string(),
            isolate: true,
        }))
        .expect("client")
    }

    #[test]
    fn config_is_validated() {
        assert!(Client::new(&ffi::Config {
            network: 9,
            ..config(no_proxy())
        })
        .is_err());
    }

    #[test]
    fn the_proxy_config_is_validated() {
        let proxy = |kind: u8, address: &str, isolate: bool| {
            super::proxy(&ffi::Proxy {
                kind,
                address: address.to_string(),
                isolate,
            })
        };
        assert_eq!(proxy(0, "", true), Ok(None));
        assert_eq!(
            proxy(1, "127.0.0.1:9050", true),
            Ok(Some(Socks5Proxy {
                endpoint: ProxyEndpoint::Tcp("127.0.0.1:9050".parse().expect("address")),
                auth: Socks5Auth::RandomPerConnection,
            }))
        );
        assert_eq!(
            proxy(1, "[::1]:9050", false),
            Ok(Some(Socks5Proxy {
                endpoint: ProxyEndpoint::Tcp("[::1]:9050".parse().expect("address")),
                auth: Socks5Auth::None,
            }))
        );
        #[cfg(unix)]
        assert_eq!(
            proxy(2, "/run/tor/socks", true),
            Ok(Some(Socks5Proxy {
                endpoint: ProxyEndpoint::Unix("/run/tor/socks".into()),
                auth: Socks5Auth::RandomPerConnection,
            }))
        );
        // Fail closed: a proxy the shell cannot use is no client at all.
        for (kind, address) in [
            (0, "127.0.0.1:9050"),
            (1, "localhost:9050"),
            (1, "127.0.0.1"),
            (1, ""),
            (2, ""),
            (7, "127.0.0.1:9050"),
        ] {
            assert!(proxy(kind, address, true).is_err(), "{kind} {address:?}");
            assert!(Client::new(&config(ffi::Proxy {
                kind,
                address: address.to_string(),
                isolate: true,
            }))
            .is_err());
        }
    }

    /// The proxy reaches every SDK the client builds, with the longer
    /// connect budget.
    #[test]
    fn a_proxied_client_builds_proxied_sdks() {
        let connect_timeout = |client: &Client| {
            client
                .set_endpoints(&["https://1.1.1.1:443".to_string()])
                .expect("endpoint");
            let sdk = client.sdk().expect("built");
            sdk.query_settings().request_settings.connect_timeout
        };
        let proxied = proxied_client();
        assert_eq!(connect_timeout(&proxied), Some(PROXIED_CONNECT_TIMEOUT));
        proxied.shutdown();
        let direct = testnet_client();
        assert_eq!(connect_timeout(&direct), Some(CONNECT_TIMEOUT));
        direct.shutdown();
    }

    /// A proxy that fails is an outage (`Unavailable`), not a node's
    /// refusal, although the SDK neither retries it nor bans the node.
    #[test]
    fn a_failing_proxy_is_unavailable_and_bans_nothing() {
        // A proxy that hangs up on every connection before answering.
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                drop(stream);
            }
        });
        let client = Client::new(&config(ffi::Proxy {
            kind: 1,
            address: address.to_string(),
            isolate: true,
        }))
        .expect("client");
        let endpoint = "https://127.0.0.1:1".to_string();
        client
            .set_endpoints(std::slice::from_ref(&endpoint))
            .expect("endpoint");
        client.provider().set_local_core_chain_locked_height(1);

        let status = crate::ops::get_identity(&client, [1u8; 32]).status;

        assert_eq!(
            status.kind,
            ffi::StatusKind::Unavailable,
            "{}",
            status.message
        );
        assert!(
            status.message.contains("SOCKS5 proxy"),
            "{}",
            status.message
        );
        let address: Address = endpoint.parse().expect("address");
        assert!(!client
            .sdk()
            .expect("sdk")
            .address_list()
            .is_banned(&address));
        client.shutdown();
    }

    /// Nothing is resolved locally, and an onion name needs the proxy.
    #[test]
    fn endpoint_hosts_are_addresses_or_proxied_onion_names() {
        let onion = "https://2gzyxa5ihm7nsggfxnu52rck2vv4rvmdlkiu3zzui5du4xyclen53wid.onion:443"
            .to_string();
        let direct = testnet_client();
        assert!(direct
            .set_endpoints(&["https://[2001:db8::1]:443".to_string()])
            .is_ok());
        assert!(direct.set_endpoints(std::slice::from_ref(&onion)).is_err());
        assert!(direct
            .set_endpoints(&["https://evo.example:443".to_string()])
            .is_err());
        direct.shutdown();

        let proxied = proxied_client();
        assert!(proxied.set_endpoints(std::slice::from_ref(&onion)).is_ok());
        assert!(proxied
            .set_endpoints(&["https://evo.example:443".to_string()])
            .is_err());
        proxied.shutdown();
    }

    /// Tasks alive on the client's runtime: each pooled tonic channel keeps
    /// one (its buffer worker, plus the connection once one is open) until
    /// the pool drops its last clone. The pool itself is private to the
    /// DAPI client, so this is how a test sees it go.
    fn alive_tasks(client: &Client) -> usize {
        client
            .runtime
            .handle()
            .expect("runtime")
            .metrics()
            .num_alive_tasks()
    }

    /// Waits up to a second for the runtime to reap finished tasks.
    fn settled_tasks(client: &Client, expected: usize) -> usize {
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            let alive = alive_tasks(client);
            if alive == expected || std::time::Instant::now() > deadline {
                return alive;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// One proved read: every attempt fails to connect and bans its address,
    /// so the retries walk the whole (two-entry) list and each endpoint ends
    /// up with a pooled channel.
    fn dial_every_endpoint(client: &Client) {
        client.provider().set_local_core_chain_locked_height(1);
        let status = crate::ops::get_identity(client, [1u8; 32]).status;
        assert_eq!(
            status.kind,
            ffi::StatusKind::Unavailable,
            "{}",
            status.message
        );
    }

    #[test]
    fn an_empty_endpoint_set_drops_the_pooled_channels() {
        let client = testnet_client();
        let uri = |port: u16| format!("https://127.0.0.1:{port}");
        let idle = alive_tasks(&client);
        client.set_endpoints(&[uri(1), uri(2)]).expect("endpoints");
        dial_every_endpoint(&client);
        assert!(
            settled_tasks(&client, idle) > idle,
            "the dialled endpoints keep pooled channels"
        );
        client.observe_protocol_version(LATEST_VERSION);
        client.observe_height(42);

        client.set_endpoints(&[]).expect("empty set");
        assert!(
            client.sdk().is_err(),
            "no SDK while there is nothing to talk to"
        );
        assert_eq!(
            settled_tasks(&client, idle),
            idle,
            "dropping the SDK closed every pooled channel"
        );

        // The next set builds a new SDK; the client's own state carried over.
        client.set_endpoints(&[uri(3)]).expect("endpoints");
        let sdk = client.sdk().expect("rebuilt");
        assert_eq!(sdk.address_list().len(), 1);
        assert_eq!(sdk.version().protocol_version, LATEST_VERSION);
        assert_eq!(client.last_seen_height(), 42);
        assert!(client.verified_platform_version().is_ok());
        client.shutdown();
    }

    #[test]
    fn an_empty_endpoint_set_stops_an_in_flight_request_dialling_the_old_nodes() {
        // Two endpoints that count the connections they get. The request's
        // first attempt is held mid-connect while the set is cleared; its
        // retry must not dial the other endpoint.
        let (accepted, dials) = std::sync::mpsc::channel();
        let uris = [(); 2].map(|()| {
            let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
            let port = listener.local_addr().expect("address").port();
            let accepted = accepted.clone();
            std::thread::spawn(move || {
                for stream in listener.incoming() {
                    if accepted.send(stream).is_err() {
                        break;
                    }
                }
            });
            format!("https://127.0.0.1:{port}")
        });
        let client = testnet_client();
        client.set_endpoints(&uris).expect("endpoints");
        let sdk = client.sdk().expect("built");
        client.provider().set_local_core_chain_locked_height(1);

        std::thread::scope(|scope| {
            let request = scope.spawn(|| crate::ops::get_identity(&client, [1u8; 32]).status);
            let first = dials
                .recv_timeout(Duration::from_secs(10))
                .expect("the request dials an endpoint");

            client.set_endpoints(&[]).expect("empty set");
            assert!(sdk.address_list().is_empty(), "the shared list is emptied");
            // The held connection fails, ending the first attempt.
            drop(first);

            let status = request.join().expect("request thread");
            assert_eq!(
                status.kind,
                ffi::StatusKind::Unavailable,
                "{}",
                status.message
            );
        });
        assert!(
            dials.recv_timeout(Duration::from_millis(500)).is_err(),
            "no further dial to a cleared endpoint"
        );
        client.shutdown();
    }

    #[test]
    fn a_shrinking_endpoint_set_drops_the_removed_channels_and_keeps_bans() {
        let client = testnet_client();
        let uri = |port: u16| format!("https://127.0.0.1:{port}");
        let idle = alive_tasks(&client);
        client.set_endpoints(&[uri(1), uri(2)]).expect("endpoints");
        let before = client.sdk().expect("built");
        dial_every_endpoint(&client);
        let dialled = settled_tasks(&client, idle);
        assert!(dialled > idle, "the dialled endpoints keep pooled channels");

        // Adding only extends the list of the same SDK.
        client
            .set_endpoints(&[uri(1), uri(2), uri(3)])
            .expect("grow");
        assert_eq!(before.address_list().len(), 3);
        drop(before);
        assert_eq!(alive_tasks(&client), dialled, "growing keeps the pool");

        // Removing one closes the pool; the retained entries keep their ban.
        client.set_endpoints(&[uri(2), uri(3)]).expect("shrink");
        assert_eq!(
            settled_tasks(&client, idle),
            idle,
            "the removed endpoint's channel is gone"
        );
        let after = client.sdk().expect("rebuilt");
        assert_eq!(after.address_list().len(), 2);
        let retained: Address = uri(2).parse().expect("address");
        assert!(
            after.address_list().is_banned(&retained),
            "ban state survives the rebuild"
        );
        client.shutdown();
    }

    #[test]
    fn endpoints_build_lazily_and_follow_the_pushed_set() {
        let client = testnet_client();
        let uri = |host: &str| format!("https://{host}:1443");
        // Nothing to talk to yet: no SDK, reads are unavailable.
        client.set_endpoints(&[]).expect("empty set before build");
        assert!(client.sdk().is_err());
        assert!(client.set_endpoints(&["not a uri".to_string()]).is_err());
        assert!(client
            .set_endpoints(&["http://1.1.1.1:1443".to_string()])
            .is_err());
        assert!(client.set_endpoints(&["1.1.1.1:1443".to_string()]).is_err());
        assert!(client.sdk().is_err(), "a bad set builds nothing");

        client
            .set_endpoints(&[uri("1.1.1.1"), uri("2.2.2.2")])
            .expect("first non-empty set builds the SDK");
        let sdk = client.sdk().expect("built");
        assert_eq!(sdk.address_list().len(), 2);

        client
            .set_endpoints(&[uri("2.2.2.2"), uri("3.3.3.3")])
            .expect("diff");
        let sdk = client.sdk().expect("rebuilt over the updated list");
        assert_eq!(sdk.address_list().len(), 2);
        assert!(sdk
            .address_list()
            .get_live_addresses()
            .iter()
            .any(|address| address.to_string().contains("3.3.3.3")));
        assert!(!sdk
            .address_list()
            .get_live_addresses()
            .iter()
            .any(|address| address.to_string().contains("1.1.1.1")));

        client
            .set_endpoints(&[])
            .expect("empty set removes everything");
        assert!(client.sdk().is_err());
        assert_eq!(
            client.check_ready().unwrap_err().kind,
            ffi::StatusKind::Unavailable
        );
        client.shutdown();
        client.shutdown();
    }

    /// The SDK decodes nothing above the response bound; the mock
    /// transport never decodes, so the setting is checked where it is
    /// made.
    #[test]
    fn responses_are_bounded_before_decoding() {
        let client = testnet_client();
        client
            .set_endpoints(&["https://1.1.1.1:1443".to_string()])
            .expect("endpoint");
        let sdk = client.sdk().expect("built");
        let settings = sdk.query_settings();
        assert_eq!(
            settings.request_settings.max_decoding_message_size,
            Some(MAX_RESPONSE_BYTES)
        );
        assert_eq!(settings.request_settings.timeout, Some(REQUEST_TIMEOUT));
        client.shutdown();
    }

    #[test]
    fn watermark_tolerates_three_blocks() {
        let client = testnet_client();
        assert!(client.observe_height(100));
        assert!(client.observe_height(98));
        assert!(client.observe_height(97));
        assert!(!client.observe_height(96));
        assert_eq!(client.last_seen_height(), 100);
        assert!(client.observe_height(104));
        assert_eq!(client.last_seen_height(), 104);
    }

    #[test]
    fn platform_version_starts_at_the_network_floor() {
        let client = testnet_client();
        assert_eq!(
            client.platform_version().protocol_version,
            min_protocol_version(Network::Testnet)
        );
    }

    #[test]
    fn transitions_wait_for_a_verified_version_unless_the_floor_is_the_latest() {
        let client = testnet_client();
        if min_protocol_version(Network::Testnet) < LATEST_VERSION {
            let error = client.verified_platform_version().unwrap_err();
            assert!(error.contains("no verified read yet"), "{error}");
            client.observe_protocol_version(LATEST_VERSION);
            assert_eq!(
                client
                    .verified_platform_version()
                    .expect("verified")
                    .protocol_version,
                LATEST_VERSION
            );
            // Upward only.
            client.observe_protocol_version(min_protocol_version(Network::Testnet));
            assert_eq!(
                client
                    .verified_platform_version()
                    .expect("verified")
                    .protocol_version,
                LATEST_VERSION
            );
        }
        // A network seen to run a version this build does not know closes
        // the builders until the embedder is updated.
        client.observe_protocol_version(LATEST_VERSION + 1);
        let error = client.verified_platform_version().unwrap_err();
        assert!(error.contains("no transition can be built"), "{error}");
        let devnet = Client::new(&ffi::Config {
            network: 2,
            ..config(no_proxy())
        })
        .expect("client");
        assert_eq!(
            devnet
                .verified_platform_version()
                .expect("the devnet floor is the latest version")
                .protocol_version,
            LATEST_VERSION
        );
    }
}
