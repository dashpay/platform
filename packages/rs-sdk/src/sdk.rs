//! [Sdk] entrypoint to Dash Platform.

use crate::error::{Error, StaleNodeError};
use crate::internal_cache::NonceCache;
use crate::mock::MockResponse;
#[cfg(feature = "mocks")]
use crate::mock::{provider::GrpcContextProvider, MockDashPlatformSdk};
use crate::platform::fetch_current_no_parameters::FetchCurrent;
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::Identifier;
use arc_swap::ArcSwapOption;
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
#[cfg(not(target_arch = "wasm32"))]
use dapi_grpc::tonic::transport::Certificate;
use dash_context_provider::ContextProvider;
#[cfg(feature = "mocks")]
use dash_context_provider::MockContextProvider;
use dpp::bincode;
use dpp::bincode::error::DecodeError;
use dpp::block::extended_epoch_info::ExtendedEpochInfo;
use dpp::dashcore::Network;
use dpp::prelude::IdentityNonce;
use dpp::version::PlatformVersion;
use drive::grovedb::operations::proof::GroveDBProof;
use drive_proof_verifier::FromProof;
pub use http::Uri;
#[cfg(feature = "mocks")]
use rs_dapi_client::mock::MockDapiClient;
pub use rs_dapi_client::Address;
pub use rs_dapi_client::AddressBanInfo;
pub use rs_dapi_client::AddressList;
pub use rs_dapi_client::RequestSettings;
use rs_dapi_client::{
    transport::TransportRequest, DapiClient, DapiClientError, DapiRequestExecutor, ExecutionResult,
};
use std::fmt::Debug;
#[cfg(feature = "mocks")]
use std::num::NonZeroUsize;
use std::path::Path;
#[cfg(feature = "mocks")]
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{atomic, Arc};
#[cfg(feature = "mocks")]
use tokio::sync::{Mutex, MutexGuard};
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};
use zeroize::Zeroizing;

/// How many data contracts fit in the cache.
pub const DEFAULT_CONTRACT_CACHE_SIZE: usize = 100;
/// How many token configs fit in the cache.
pub const DEFAULT_TOKEN_CONFIG_CACHE_SIZE: usize = 100;
/// How many quorum public keys fit in the cache.
pub const DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE: usize = 100;
/// Per-network *default* seed used only when an unpinned SDK has no explicit
/// initial version.
///
/// Not a runtime clamp: [`SdkBuilder::with_initial_version`] can seed an unpinned
/// SDK *below* this value (no construction-time floor), and auto-detect
/// ([`Sdk::maybe_update_protocol_version`]) only ratchets the stored version
/// *upward* via `fetch_max` when the network reports a newer one.
const fn min_protocol_version(network: Network) -> u32 {
    match network {
        Network::Mainnet => dpp::version::v11::PROTOCOL_VERSION_11,
        Network::Testnet => dpp::version::v12::PROTOCOL_VERSION_12,
        Network::Devnet => dpp::version::v12::PROTOCOL_VERSION_12,
        Network::Regtest => dpp::version::v12::PROTOCOL_VERSION_12,
    }
}

/// Default signed-metadata freshness window for network SDKs.
const DEFAULT_METADATA_TIME_TOLERANCE_MS: u64 = 31 * 60 * 1000;

/// The default request settings for the SDK, used when the user does not provide any.
///
/// Use [SdkBuilder::with_settings] to set custom settings.
const DEFAULT_REQUEST_SETTINGS: RequestSettings = RequestSettings {
    retries: Some(3),
    timeout: None,
    ban_failed_address: None,
    connect_timeout: None,
    max_decoding_message_size: None,
};

/// Build the default DAPI bootstrap address list for `network` from
/// [`dash_network_seeds`].
///
/// The seed lists are single-source-of-truth, weekly-refreshed upstream in
/// `rust-dashcore`. We filter to Evo (HPMN) masternodes — the only ones that
/// run Dash Platform — and build `https://<ip>:<platform_http_port>` URIs.
/// The Core port on `seed.address` is intentionally discarded: DAPI clients
/// need the platform HTTP port, not the Core P2P port.
///
/// Malformed upstream entries are silently skipped rather than panicking;
/// the DAPI client handles retry/rotation across the remaining addresses.
///
/// Seeds whose recorded Platform TLS probe shows a certificate that this
/// client's rustls stack would deterministically reject (`Expired`,
/// `SelfSigned`, `Untrusted`) are skipped: every connect to them fails the
/// handshake, so keeping them in rotation only costs retry/ban churn.
/// `NoHandshake` is skipped only when the probe's TCP connect succeeded
/// (`reachable == Ok`) — the prober also stamps `NoHandshake` on TCP
/// timeouts and probe-budget expiry, which are transient conditions best
/// left to runtime banning. `Valid` and `Unknown` (not probed) are kept. If the
/// filter would empty the list (e.g. a seed file with all-stale probes),
/// it falls back to the unfiltered set so the client can still bootstrap
/// and let runtime banning sort it out.
///
/// ## Panics
///
/// Panics on networks other than `Mainnet` and `Testnet` — no upstream
/// seed list exists for devnet/regtest.
fn default_address_list_for_network(network: Network) -> AddressList {
    if !matches!(network, Network::Mainnet | Network::Testnet) {
        panic!("default address list is only available for mainnet and testnet");
    }

    let seeds = dash_network_seeds::evo_seeds(network);
    let filtered = address_list_from_seeds(&seeds, true);
    if filtered.is_empty() {
        tracing::warn!(
            ?network,
            "all seed entries have failing TLS probes; falling back to unfiltered seed list"
        );
        return address_list_from_seeds(&seeds, false);
    }
    filtered
}

/// Whether a seed's recorded Platform TLS probe is a failure this client
/// would deterministically reproduce on every connect. `NoHandshake` is
/// also stamped by the prober on TCP timeout / probe-budget expiry, which
/// are transient — it only counts when the probe's TCP connect itself
/// succeeded. An unprobed seed (`None` / `Unknown`) is never rejected.
fn seed_tls_deterministically_bad(platform: Option<&dash_network_seeds::PlatformStatus>) -> bool {
    use dash_network_seeds::{Reachability, SslStatus};
    let Some(platform) = platform else {
        return false;
    };
    match platform.ssl {
        SslStatus::Expired | SslStatus::SelfSigned | SslStatus::Untrusted => true,
        SslStatus::NoHandshake => platform.reachable == Reachability::Ok,
        SslStatus::Valid | SslStatus::Unknown => false,
    }
}

/// Build an [`AddressList`] of `https://<ip>:<platform_http_port>` entries
/// from `seeds`, optionally skipping seeds whose TLS probe is a
/// deterministic failure (see [`seed_tls_deterministically_bad`]).
fn address_list_from_seeds(
    seeds: &[dash_network_seeds::MasternodeSeed],
    skip_bad_tls: bool,
) -> AddressList {
    let mut list = AddressList::new();
    for seed in seeds {
        let Some(port) = seed.platform_http_port else {
            continue;
        };
        if skip_bad_tls && seed_tls_deterministically_bad(seed.platform.as_ref()) {
            continue;
        }
        let url = format!("https://{}:{}", seed.address.ip(), port);
        if let Ok(uri) = url.parse::<Uri>() {
            if let Ok(address) = Address::try_from(uri) {
                list.add(address);
            }
        }
    }
    list
}

/// Dash Platform SDK
///
/// This is the main entry point for interacting with Dash Platform.
/// It can be initialized in two modes:
/// - `Normal`: Connects to a remote Dash Platform node.
/// - `Mock`: Uses a mock implementation of Dash Platform.
///
/// Recommended method of initialization is to use [`SdkBuilder`]. There are also some helper
/// methods:
///
/// * [`SdkBuilder::new_testnet()`] Create a [SdkBuilder] that connects to testnet.
/// * [`SdkBuilder::new_mainnet()`] Create a [SdkBuilder] that connects to mainnet.
/// * [`SdkBuilder::new_mock()`] Create a mock [SdkBuilder].
/// * [`Sdk::new_mock()`] Create a mock [Sdk].
///
/// ## Thread safety
///
/// Sdk is thread safe and can be shared between threads.
/// It uses internal locking when needed.
///
/// It is also safe to clone the Sdk.
///
/// ## Examples
///
/// See tests/ for examples of using the SDK.
pub struct Sdk {
    /// The network that the sdk is configured for (Dash (mainnet), Testnet, Devnet, Regtest)
    pub network: Network,
    inner: SdkInstance,
    /// Use proofs when retrieving data from Platform.
    ///
    /// This is set to `true` by default. `false` is not implemented yet.
    proofs: bool,

    /// Nonce cache managed exclusively by the SDK.
    nonce_cache: Arc<NonceCache>,

    /// Context provider used by the SDK.
    ///
    /// ## Panics
    ///
    /// Note that setting this to None can panic.
    context_provider: ArcSwapOption<Box<dyn ContextProvider>>,

    /// Protocol version number detected from the network. Shared between clones.
    protocol_version: Arc<atomic::AtomicU32>,

    /// Whether the protocol version is pinned, i.e. auto-detection from network
    /// response metadata is disabled. Set to `true` when the user explicitly calls
    /// [`SdkBuilder::with_version()`].
    version_pinned: bool,

    /// Last seen height; used to determine if the remote node is stale.
    ///
    /// This is clone-able and can be shared between threads.
    metadata_last_seen_height: Arc<atomic::AtomicU64>,

    /// How many blocks difference is allowed between the last height and the current height received in metadata.
    ///
    /// See [SdkBuilder::with_height_tolerance] for more information.
    metadata_height_tolerance: Option<u64>,

    /// How many milliseconds difference is allowed between the time received in response and current local time.
    ///
    /// See [SdkBuilder::with_time_tolerance] for more information.
    metadata_time_tolerance_ms: Option<u64>,

    /// Cancellation token; once cancelled, all pending requests should be aborted.
    pub(crate) cancel_token: CancellationToken,

    /// Global settings of dapi client
    pub(crate) dapi_client_settings: RequestSettings,

    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,
}
impl Clone for Sdk {
    fn clone(&self) -> Self {
        Self {
            network: self.network,
            inner: self.inner.clone(),
            proofs: self.proofs,
            nonce_cache: Arc::clone(&self.nonce_cache),
            context_provider: ArcSwapOption::new(self.context_provider.load_full()),
            cancel_token: self.cancel_token.clone(),
            protocol_version: Arc::clone(&self.protocol_version),
            version_pinned: self.version_pinned,
            metadata_last_seen_height: Arc::clone(&self.metadata_last_seen_height),
            metadata_height_tolerance: self.metadata_height_tolerance,
            metadata_time_tolerance_ms: self.metadata_time_tolerance_ms,
            dapi_client_settings: self.dapi_client_settings,
            #[cfg(feature = "mocks")]
            dump_dir: self.dump_dir.clone(),
        }
    }
}

impl Debug for Sdk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.inner {
            SdkInstance::Dapi { dapi, .. } => f
                .debug_struct("Sdk")
                .field("dapi", dapi)
                .field("proofs", &self.proofs)
                .finish(),
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { mock, .. } => f
                .debug_struct("Sdk")
                .field("mock", mock)
                .field("proofs", &self.proofs)
                .finish(),
        }
    }
}

/// Internal Sdk instance.
///
/// This is used to store the actual Sdk instance, which can be either a real Sdk or a mock Sdk.
/// We use it to avoid exposing internals defined below to the public.
#[derive(Debug, Clone)]
enum SdkInstance {
    /// Real Sdk, using DAPI with gRPC transport
    Dapi {
        /// DAPI client used to communicate with Dash Platform.
        dapi: DapiClient,
    },
    /// Mock SDK
    #[cfg(feature = "mocks")]
    Mock {
        /// Mock DAPI client used to communicate with Dash Platform.
        ///
        /// Dapi client is wrapped in a tokio [Mutex](tokio::sync::Mutex) as it's used in async context.
        dapi: Arc<Mutex<MockDapiClient>>,
        /// Mock SDK implementation processing mock expectations and responses.
        mock: Arc<Mutex<MockDashPlatformSdk>>,
        address_list: AddressList,
    },
}

impl Sdk {
    /// Initialize Dash Platform SDK in mock mode.
    ///
    /// This is a helper method that uses [`SdkBuilder`] to initialize the SDK in mock mode.
    ///
    /// See also [`SdkBuilder`].
    pub fn new_mock() -> Self {
        SdkBuilder::default()
            .build()
            .expect("mock should be created")
    }

    /// Return freshness criteria (height tolerance and time tolerance) for given request method.
    ///
    /// Note that if self.metadata_height_tolerance or self.metadata_time_tolerance_ms is None,
    /// respective tolerance will be None regardless of method, to allow disabling staleness checks globally.
    fn freshness_criteria(&self, method_name: &str) -> (Option<u64>, Option<u64>) {
        match method_name {
            "get_addresses_trunk_state" | "get_addresses_branch_state" => (
                // Address synchronization checkpoints can lag the latest
                // Platform height. Prefer their signed time when available,
                // but retain the independently trusted height floor for the
                // explicitly supported height-only configuration.
                self.metadata_time_tolerance_ms
                    .is_none()
                    .then_some(self.metadata_height_tolerance)
                    .flatten(),
                self.metadata_time_tolerance_ms
                    .map(|configured| configured.min(DEFAULT_METADATA_TIME_TOLERANCE_MS)),
            ),
            _ => (
                self.metadata_height_tolerance,
                self.metadata_time_tolerance_ms,
            ),
        }
    }

    /// Verify response metadata against the current state of the SDK.
    pub fn verify_response_metadata(
        &self,
        method_name: &str,
        metadata: &ResponseMetadata,
    ) -> Result<(), Error> {
        let (metadata_height_tolerance, metadata_time_tolerance_ms) =
            self.freshness_criteria(method_name);
        // Check the independent local-clock anchor before mutating the
        // response-derived height high-water mark.
        if let Some(time_tolerance) = metadata_time_tolerance_ms {
            let now = chrono::Utc::now().timestamp_millis() as u64;
            verify_metadata_time(metadata, now, time_tolerance)?;
        };
        if let Some(height_tolerance) = metadata_height_tolerance {
            verify_metadata_height(
                metadata,
                height_tolerance,
                Arc::clone(&(self.metadata_last_seen_height)),
            )?;
        };

        self.maybe_update_protocol_version(metadata.protocol_version);

        Ok(())
    }

    /// Update the stored protocol version if `received_version` is newer and known.
    ///
    /// Uses `fetch_max` so the highest version always wins under concurrent updates.
    /// The version is stored per-SDK instance (not in the process-wide global),
    /// so multiple SDK instances can track different networks independently.
    fn maybe_update_protocol_version(&self, received_version: u32) {
        if self.version_pinned {
            return;
        }

        if received_version == 0 {
            return;
        }

        let current = self.protocol_version.load(Ordering::Relaxed);

        if received_version <= current {
            return;
        }

        // Validate that we know this version before accepting it
        if PlatformVersion::get(received_version).is_err() {
            tracing::warn!(
                received_version,
                current_version = current,
                "received unknown protocol version from network; keeping current"
            );
            return;
        }

        let previous = self
            .protocol_version
            .fetch_max(received_version, Ordering::Relaxed);
        if previous < received_version {
            tracing::info!(
                target: "dash_sdk::protocol_version",
                from = previous,
                to = received_version,
                "ratcheting protocol version upward"
            );
        }
    }

    /// Eagerly teach this SDK the network's current protocol version and ratchet up to it.
    ///
    /// Issues ordinary **proven** `getEpochsInfo` queries
    /// ([`ExtendedEpochInfo::fetch_current`]) and discards the epoch payload. The
    /// protocol version those queries carry in their verified response metadata is
    /// ratcheted in by the *same* [`Self::maybe_update_protocol_version`] path
    /// every other query uses — only after proof + quorum-signature verification
    /// succeeds. Refresh therefore inherits the exact cryptographic trust of
    /// ordinary traffic; it adds no second, weaker source of truth.
    ///
    /// On a pinned SDK ([`SdkBuilder::with_version`], `version_pinned`
    /// on) this issues no request and returns the pinned version.
    ///
    /// If the fetch fails the failure is **non-fatal**: whatever version was
    /// already learned is kept — we never fall back to an unverified one. Note
    /// that [`ExtendedEpochInfo::fetch_current`] makes more than one round trip,
    /// and each verified response ratchets the version on its own. A refresh that
    /// ends in an error may therefore still have raised the stored version, and
    /// the value returned here reflects that. This is by construction: every
    /// ratchet step is proof-verified and upward-only, so a partial refresh can
    /// only ever leave the SDK closer to the network's real version.
    ///
    /// On a proofs-disabled SDK ([`SdkBuilder::with_proofs`]`(false)`) this is a
    /// no-op that returns the current version: refresh relies on a proven query,
    /// so with proofs off there is no trusted source to ratchet from.
    ///
    /// Returns the SDK's protocol version number after the (possible) ratchet.
    ///
    /// [`SdkBuilder::with_version`]: SdkBuilder::with_version
    pub async fn refresh_protocol_version(&self) -> Result<u32, Error> {
        if !self.prove() {
            return Ok(self.protocol_version_number());
        }
        if !self.version_pinned {
            if let Err(error) = ExtendedEpochInfo::fetch_current(self).await {
                tracing::warn!(
                    target: "dash_sdk::protocol_version",
                    %error,
                    version = self.protocol_version_number(),
                    "proven protocol-version refresh failed; keeping the highest \
                     proof-verified version learned so far (never falling back to \
                     an unverified one)"
                );
            }
        }
        Ok(self.protocol_version_number())
    }

    /// Retrieve object `O` from proof contained in `request` (of type `R`) and `response`.
    ///
    /// This method is used to retrieve objects from proofs returned by Dash Platform.
    ///
    /// ## Generic Parameters
    ///
    /// - `R`: Type of the request that was used to fetch the proof.
    /// - `O`: Type of the object to be retrieved from the proof.
    ///
    /// ## Protocol version bootstrapping
    ///
    /// On a fresh auto-detect SDK (i.e. one built without [`SdkBuilder::with_version()`]), the
    /// first call to this method uses the per-network [`min_protocol_version`] floor as a fallback
    /// because no network response has been received yet to teach the SDK the real network version.
    ///
    /// The actual network version is learned only *after* proof parsing succeeds, when
    /// [`Self::verify_response_metadata()`] processes `metadata.protocol_version`.  If the
    /// connected network runs an older protocol version **and** proof interpretation differs
    /// between that version and the seeded [`min_protocol_version`], the very first request may
    /// fail before the SDK can correct itself.  Subsequent requests will use the correct version.
    ///
    /// This is a known bootstrap limitation.  Callers that must guarantee correct version
    /// behaviour on the first request should pin the version explicitly via
    /// [`SdkBuilder::with_version()`].
    pub(crate) async fn parse_proof_with_metadata_and_proof<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
        method_name: &'static str,
    ) -> Result<(Option<O>, ResponseMetadata, Proof), Error>
    where
        O::Request: Mockable,
    {
        let provider = self
            .context_provider()
            .ok_or(drive_proof_verifier::Error::ContextProviderNotSet)?;

        let (object, metadata, proof) = match self.inner {
            SdkInstance::Dapi { .. } => O::maybe_from_proof_with_metadata(
                request,
                response,
                self.network,
                self.version(),
                &provider,
            ),
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mock, .. } => {
                let guard = mock.lock().await;
                guard.parse_proof_with_metadata(request, response)
            }
        }?;

        // Security invariant: proof+signature verification above (the `?`) must
        // precede this call, which ratchets the protocol version from the now-trusted
        // `metadata.protocol_version`. Never reorder — the ratchet must not consume
        // unverified metadata.
        self.verify_response_metadata(method_name, &metadata)
            .inspect_err(|err| {
                tracing::warn!(%err,method=method_name,"received response with stale metadata; try another server");
            })?;

        Ok((object, metadata, proof))
    }

    /// Return [ContextProvider] used by the SDK.
    pub fn context_provider(&self) -> Option<impl ContextProvider> {
        let provider_guard = self.context_provider.load();
        let provider = provider_guard.as_ref().map(Arc::clone);

        provider
    }

    /// Returns a mutable reference to the `MockDashPlatformSdk` instance.
    ///
    /// Use returned object to configure mock responses with methods like `expect_fetch`.
    ///
    /// # Panics
    ///
    /// Panics when:
    ///
    /// * the `self` instance is not a `Mock` variant,
    /// * the `self` instance is in use by another thread.
    #[cfg(feature = "mocks")]
    pub fn mock(&mut self) -> MutexGuard<'_, MockDashPlatformSdk> {
        if let Sdk {
            inner: SdkInstance::Mock { ref mock, .. },
            ..
        } = self
        {
            mock.try_lock()
                .expect("mock sdk is in use by another thread and cannot be reconfigured")
        } else {
            panic!("not a mock")
        }
    }

    /// Get or fetch identity nonce, querying Platform when stale or absent.
    /// Treats a missing nonce as `0` before applying the optional bump; on first
    /// interaction this may return `0` or `1` depending on `bump_first`. Does not
    /// verify identity existence.
    pub async fn get_identity_nonce(
        &self,
        identity_id: Identifier,
        bump_first: bool,
        settings: Option<PutSettings>,
    ) -> Result<IdentityNonce, Error> {
        let settings = settings.unwrap_or_default();
        let nonce = self
            .nonce_cache
            .get_identity_nonce(self, identity_id, bump_first, &settings)
            .await?;

        tracing::trace!(
            identity_id = %identity_id,
            bump_first,
            nonce,
            "Fetched identity nonce"
        );

        Ok(nonce)
    }

    /// Get or fetch identity-contract nonce, querying Platform when stale or absent.
    /// Treats a missing nonce as `0` before applying the optional bump; on first
    /// interaction this may return `0` or `1` depending on `bump_first`. Does not
    /// verify identity or contract existence.
    pub async fn get_identity_contract_nonce(
        &self,
        identity_id: Identifier,
        contract_id: Identifier,
        bump_first: bool,
        settings: Option<PutSettings>,
    ) -> Result<IdentityNonce, Error> {
        let settings = settings.unwrap_or_default();
        self.nonce_cache
            .get_identity_contract_nonce(self, identity_id, contract_id, bump_first, &settings)
            .await
    }

    /// Marks identity nonce cache entries as stale so they are re-fetched from
    /// Platform on the next call to [`get_identity_nonce`] or
    /// [`get_identity_contract_nonce`].
    pub async fn refresh_identity_nonce(&self, identity_id: &Identifier) {
        self.nonce_cache.refresh(identity_id).await;
    }

    /// Return [Dash Platform version](PlatformVersion) information used by this SDK.
    ///
    /// With auto-detection (default) the SDK starts at the per-network
    /// [`min_protocol_version`] (or the seed set via
    /// [`SdkBuilder::with_initial_version`]) and then tracks the network's version
    /// — auto-detection only ever ratchets *upward* (`fetch_max`). A version pinned
    /// via [`SdkBuilder::with_version()`] is returned as pinned.
    pub fn version<'v>(&self) -> &'v PlatformVersion {
        let v = self.protocol_version.load(Ordering::Relaxed);
        PlatformVersion::get(v).unwrap_or_else(|_| PlatformVersion::latest())
    }

    /// Return the raw protocol version number currently used by this SDK.
    pub fn protocol_version_number(&self) -> u32 {
        self.protocol_version.load(Ordering::Relaxed)
    }

    // TODO: Move to settings
    /// Indicate if the sdk should request and verify proofs.
    pub fn prove(&self) -> bool {
        self.proofs
    }

    /// Build a [`QuerySettings`] borrowing this SDK's protocol version,
    /// request settings, and `prove` flag.
    ///
    /// Hand the resulting context to [`crate::platform::Query::query`] when
    /// you need to encode a user-facing query into a wire `TransportRequest`
    /// without taking a full `&Sdk` dependency through the encoder layer.
    pub fn query_settings(&self) -> crate::platform::QuerySettings<'_> {
        crate::platform::QuerySettings {
            request_settings: &self.dapi_client_settings,
            protocol_version: self.version(),
            prove: self.prove(),
        }
    }

    // TODO: If we remove this setter we don't need to use ArcSwap.
    //   It's good enough to set Context once when you initialize the SDK.
    /// Set the [ContextProvider] to use.
    ///
    /// [ContextProvider] is used to access state information, like data contracts and quorum public keys.
    ///
    /// Note that this will overwrite any previous context provider.
    pub fn set_context_provider<C: ContextProvider + 'static>(&self, context_provider: C) {
        self.context_provider
            .swap(Some(Arc::new(Box::new(context_provider))));
    }

    /// Returns a future that resolves when the Sdk is cancelled (e.g. shutdown was requested).
    pub fn cancelled(&self) -> WaitForCancellationFuture<'_> {
        self.cancel_token.cancelled()
    }

    /// Request shutdown of the Sdk and all related operations.
    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }

    /// Return the [DapiClient] address list
    pub fn address_list(&self) -> &AddressList {
        match &self.inner {
            SdkInstance::Dapi { dapi, .. } => dapi.address_list(),
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { address_list, .. } => address_list,
        }
    }

    /// Return an owned snapshot of every DAPI address' ban state,
    /// including the reason the address was banned (when recorded).
    ///
    /// Delegates to [`AddressList::ban_info`]. Useful for diagnostics
    /// and surfacing ban state up through the platform-wallet FFI to
    /// the iOS example app.
    pub fn address_ban_info(&self) -> Vec<AddressBanInfo> {
        self.address_list().ban_info()
    }
}

/// If received metadata time differs from local time by more than `tolerance`, the remote node is considered stale.
///
/// ## Parameters
///
/// - `metadata`: Metadata of the received response
/// - `now_ms`: Current local time in milliseconds
/// - `tolerance_ms`: Tolerance in milliseconds
pub(crate) fn verify_metadata_time(
    metadata: &ResponseMetadata,
    now_ms: u64,
    tolerance_ms: u64,
) -> Result<(), Error> {
    let metadata_time = metadata.time_ms;

    // metadata_time - tolerance_ms <= now_ms <= metadata_time + tolerance_ms
    if now_ms.abs_diff(metadata_time) > tolerance_ms {
        return Err(StaleNodeError::Time {
            expected_timestamp_ms: now_ms,
            received_timestamp_ms: metadata_time,
            tolerance_ms,
        }
        .into());
    }

    tracing::trace!(
        expected_time = now_ms,
        received_time = metadata_time,
        tolerance_ms,
        "received response with valid time"
    );
    Ok(())
}

/// If current metadata height is behind previously seen height by more than `tolerance`, the remote node
///  is considered stale.
fn verify_metadata_height(
    metadata: &ResponseMetadata,
    tolerance: u64,
    last_seen_height: Arc<atomic::AtomicU64>,
) -> Result<(), Error> {
    let received_height = metadata.height;
    // Linearize the response at an atomic max update, then reload so a racing
    // higher response that committed before this validation completes is also
    // considered. A lower accepted response can never reduce the baseline.
    let previous_height = last_seen_height.fetch_max(received_height, Ordering::AcqRel);
    let expected_height = previous_height.max(last_seen_height.load(Ordering::Acquire));

    if expected_height > tolerance && received_height < expected_height.saturating_sub(tolerance) {
        return Err(StaleNodeError::Height {
            expected_height,
            received_height,
            tolerance_blocks: tolerance,
        }
        .into());
    }

    tracing::trace!(
        expected_height,
        received_height,
        tolerance,
        "received response within the monotonic height window"
    );

    Ok(())
}

#[async_trait::async_trait]
impl DapiRequestExecutor for Sdk {
    async fn execute<R: TransportRequest>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> ExecutionResult<R::Response, DapiClientError> {
        match self.inner {
            SdkInstance::Dapi { ref dapi, .. } => dapi.execute(request, settings).await,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref dapi, .. } => {
                let dapi_guard = dapi.lock().await;
                dapi_guard.execute(request, settings).await
            }
        }
    }
}

/// Dash Platform SDK Builder, used to configure and [`SdkBuilder::build()`] the [Sdk].
///
/// [SdkBuilder] implements a "builder" design pattern to allow configuration of the Sdk before it is instantiated.
/// It allows creation of Sdk in two modes:
/// - `Normal`: Connects to a remote Dash Platform node.
/// - `Mock`: Uses a mock implementation of Dash Platform.
///
/// Mandatory steps of initialization in normal mode are:
///
/// 1. Create an instance of [SdkBuilder] with [`SdkBuilder::new()`]
/// 2. Configure the builder with [`SdkBuilder::with_core()`]
/// 3. Call [`SdkBuilder::build()`] to create the [Sdk] instance.
pub struct SdkBuilder {
    /// List of addresses to connect to.
    ///
    /// If `None`, a mock client will be created.
    addresses: Option<AddressList>,
    settings: Option<RequestSettings>,

    network: Network,

    core_ip: String,
    core_port: u16,
    core_user: String,
    core_password: Zeroizing<String>,

    /// If true, request and verify proofs of the responses.
    proofs: bool,

    /// Platform version to use in this Sdk; if None, the SDK will auto-detect the version
    /// from network metadata and update it as needed.
    version: Option<&'static PlatformVersion>,

    /// Whether the protocol version is pinned, i.e. the user explicitly called
    /// `with_version()`. When true, auto-detection of protocol version from network
    /// metadata is disabled.
    version_pinned: bool,

    /// Cache size for data contracts. Used by mock [GrpcContextProvider].
    #[cfg(feature = "mocks")]
    data_contract_cache_size: NonZeroUsize,

    /// Cache size for token configs. Used by mock [GrpcContextProvider].
    #[cfg(feature = "mocks")]
    token_config_cache_size: NonZeroUsize,

    /// Cache size for quorum public keys. Used by mock [GrpcContextProvider].
    #[cfg(feature = "mocks")]
    quorum_public_keys_cache_size: NonZeroUsize,

    /// Context provider used by the SDK.
    context_provider: Option<Box<dyn ContextProvider>>,

    /// How many blocks difference is allowed between the last seen metadata height and the height received in response
    /// metadata.
    ///
    /// See [SdkBuilder::with_height_tolerance] for more information.
    metadata_height_tolerance: Option<u64>,

    /// How many milliseconds difference is allowed between the time received in response metadata and current local time.
    ///
    /// See [SdkBuilder::with_time_tolerance] for more information.
    metadata_time_tolerance_ms: Option<u64>,

    /// Independently trusted initial Platform height used to seed the
    /// monotonic freshness high-water mark.
    trusted_initial_height: Option<u64>,

    /// directory where dump files will be stored
    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,

    /// Cancellation token; once cancelled, all pending requests should be aborted.
    pub(crate) cancel_token: CancellationToken,

    /// CA certificate to use for TLS connections.
    #[cfg(not(target_arch = "wasm32"))]
    ca_certificate: Option<Certificate>,
}

impl Default for SdkBuilder {
    /// Create default SdkBuilder that will create a mock client.
    fn default() -> Self {
        Self {
            addresses: None,
            settings: None,
            network: Network::Mainnet,
            core_ip: "".to_string(),
            core_port: 0,
            core_password: "".to_string().into(),
            core_user: "".to_string(),

            proofs: true,
            metadata_height_tolerance: Some(1),
            metadata_time_tolerance_ms: None,
            trusted_initial_height: None,

            #[cfg(feature = "mocks")]
            data_contract_cache_size: NonZeroUsize::new(DEFAULT_CONTRACT_CACHE_SIZE)
                .expect("data contract cache size must be positive"),

            #[cfg(feature = "mocks")]
            token_config_cache_size: NonZeroUsize::new(DEFAULT_TOKEN_CONFIG_CACHE_SIZE)
                .expect("token config cache size must be positive"),

            #[cfg(feature = "mocks")]
            quorum_public_keys_cache_size: NonZeroUsize::new(DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE)
                .expect("quorum public keys cache size must be positive"),

            context_provider: None,

            cancel_token: CancellationToken::new(),

            // No version configured; `build()` defaults to the per-network
            // `min_protocol_version` unless `with_version`/`with_initial_version`
            // sets one.
            version: None,
            version_pinned: false,
            #[cfg(not(target_arch = "wasm32"))]
            ca_certificate: None,

            #[cfg(feature = "mocks")]
            dump_dir: None,
        }
    }
}

impl SdkBuilder {
    /// Enable or disable proofs on requests.
    ///
    /// In mock/offline testing with recorded vectors, set to false to match dumps
    /// that were captured without proofs.
    pub fn with_proofs(mut self, proofs: bool) -> Self {
        self.proofs = proofs;
        self
    }
    /// Create a new SdkBuilder with provided address list.
    pub fn new(addresses: AddressList) -> Self {
        Self {
            addresses: Some(addresses),
            metadata_time_tolerance_ms: Some(DEFAULT_METADATA_TIME_TOLERANCE_MS),
            ..Default::default()
        }
    }

    /// Replace the address list on this builder.
    pub fn with_address_list(mut self, addresses: AddressList) -> Self {
        self.addresses = Some(addresses);
        self
    }

    /// Create a new SdkBuilder that will generate mock client.
    pub fn new_mock() -> Self {
        Self::default()
    }

    /// Create a new SdkBuilder instance preconfigured for testnet.
    ///
    /// This is a helper method that preconfigures [SdkBuilder] for testnet use.
    /// Use this method if you want to connect to Dash Platform testnet during development and testing
    /// of your solution.
    pub fn new_testnet() -> Self {
        let address_list = default_address_list_for_network(Network::Testnet);

        Self::new(address_list).with_network(Network::Testnet)
    }

    /// Create a new SdkBuilder instance preconfigured for mainnet (production network).
    ///
    /// This is a helper method that preconfigures [SdkBuilder] for production use.
    /// Use this method if you want to connect to Dash Platform mainnet with production-ready product.
    ///
    /// ## Panics
    ///
    /// This method panics if the mainnet configuration cannot be loaded.
    ///
    /// ## Unstable
    ///
    /// This method is unstable and can be changed in the future.
    pub fn new_mainnet() -> Self {
        let address_list = default_address_list_for_network(Network::Mainnet);

        Self::new(address_list).with_network(Network::Mainnet)
    }

    /// Configure network type.
    ///
    /// Defaults to Network::Mainnet which is mainnet.
    pub fn with_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// Configure CA certificate to use when verifying TLS connections.
    ///
    /// Used mainly for testing purposes and local networks.
    ///
    /// If not set, uses standard system CA certificates.
    ///
    /// ## Parameters
    ///
    /// - `pem_certificate`: PEM-encoded CA certificate. User must ensure that the certificate is valid.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_ca_certificate(mut self, pem_certificate: Certificate) -> Self {
        self.ca_certificate = Some(pem_certificate);
        self
    }

    /// Load CA certificate from a PEM-encoded file.
    ///
    /// This is a convenience method that reads the certificate from a file and sets it using
    /// [SdkBuilder::with_ca_certificate()].
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_ca_certificate_file(
        self,
        certificate_file_path: impl AsRef<Path>,
    ) -> std::io::Result<Self> {
        let pem = std::fs::read(certificate_file_path)?;
        let cert = Certificate::from_pem(pem);

        Ok(self.with_ca_certificate(cert))
    }

    /// Configure request settings.
    ///
    /// Tune request settings used to connect to the Dash Platform.
    ///
    /// Defaults to [`DEFAULT_REQUEST_SETTINGS`], which sets retries to 3.
    ///
    /// See [`RequestSettings`] for more information.
    pub fn with_settings(mut self, settings: RequestSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Configure platform version.
    ///
    /// Select specific version of Dash Platform to use. This pins the version and
    /// disables auto-detection.
    ///
    /// The pinned version is used as-is; it is not clamped to the per-network
    /// [`min_protocol_version`].
    ///
    /// When unset, the SDK starts at the per-network [`min_protocol_version`] and
    /// ratchets upward via auto-detection.
    pub fn with_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = Some(version);
        self.version_pinned = true;
        self
    }

    /// Override the initial protocol version seed while keeping auto-detect on.
    ///
    /// Unpinned SDKs otherwise seed at the per-network [`min_protocol_version`] and
    /// ratchet upward via `fetch_max` in `maybe_update_protocol_version` once the
    /// network's version is observed. This replaces that seed with `version`.
    ///
    /// The seed is used verbatim — including versions *below* the per-network floor
    /// (no construction-time clamp; configuring a valid seed is the caller's
    /// responsibility). A sub-floor seed is only corrected once a proven response
    /// ratchets the version upward; callers needing eager on-init discovery should
    /// call [`Sdk::refresh_protocol_version`] after building.
    ///
    /// Seeds `self.version` and keeps `version_pinned` `false`, so auto-detect stays
    /// on. Builder chains are last-write-wins: a later `with_initial_version` re-enables
    /// auto-detect that an earlier `with_version` disabled.
    pub fn with_initial_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = Some(version);
        self.version_pinned = false;
        self
    }

    /// Configure context provider to use.
    ///
    /// Context provider is used to retrieve data contracts and quorum public keys from application state.
    /// It should be implemented by the user of this SDK to provide stateful information about the application.
    ///
    /// See [ContextProvider] for more information and [GrpcContextProvider] for an example implementation.
    pub fn with_context_provider<C: ContextProvider + 'static>(
        mut self,
        context_provider: C,
    ) -> Self {
        self.context_provider = Some(Box::new(context_provider));

        self
    }

    /// Set cancellation token that will be used by the Sdk.
    ///
    /// Once that cancellation token is cancelled, all pending requests shall terminate.
    pub fn with_cancellation_token(mut self, cancel_token: CancellationToken) -> Self {
        self.cancel_token = cancel_token;
        self
    }

    /// Use Dash Core as a wallet and context provider.
    ///
    /// This is a convenience method that configures the SDK to use Dash Core as a wallet and context provider.
    ///
    /// For more control over the configuration, use [`SdkBuilder::with_context_provider()`].
    ///
    /// This is temporary implementation, intended for development purposes.
    pub fn with_core(mut self, ip: &str, port: u16, user: &str, password: &str) -> Self {
        self.core_ip = ip.to_string();
        self.core_port = port;
        self.core_user = user.to_string();
        self.core_password = Zeroizing::from(password.to_string());

        self
    }

    /// Change number of blocks difference allowed between the last height and the height received in current response.
    ///
    /// If height received in response metadata is behind previously seen height by more than this value, the node
    /// is considered stale, and the request will fail.
    ///
    /// If None, the height is not checked.
    ///
    /// Note that this feature doesn't guarantee that you are getting latest data, but it significantly decreases
    /// probability of getting old data.
    ///
    /// This is set to `1` by default.
    pub fn with_height_tolerance(mut self, tolerance: Option<u64>) -> Self {
        self.metadata_height_tolerance = tolerance;
        self
    }

    /// How many milliseconds difference is allowed between the time received in response and current local time.
    /// If the received time differs from local time by more than this value, the remote node is stale.
    ///
    /// If None, the time is not checked.
    ///
    /// Network builders default to 31 minutes. Mock builders default to
    /// `None`. Disabling this for a proof-enabled network SDK requires a
    /// trusted initial height with height checking enabled.
    ///
    /// Note that enabling this check can cause issues if the local time is not synchronized with the network time,
    /// when the network is stalled or time between blocks increases significantly.
    ///
    /// Selecting a safe value for this parameter depends on maximum time between blocks mined on the network.
    /// For example, if the network is configured to mine a block every maximum 3 minutes, setting this value
    /// to a bit more than 6 minutes (to account for misbehaving proposers, network delays and local time
    /// synchronization issues) should be safe.
    pub fn with_time_tolerance(mut self, tolerance_ms: Option<u64>) -> Self {
        self.metadata_time_tolerance_ms = tolerance_ms;
        self
    }

    /// Seed proof freshness with an independently trusted Platform height.
    ///
    /// This can be used instead of the local-clock policy. The checkpoint must
    /// come from a trusted source and should be persisted with its network and
    /// provenance by the caller.
    pub fn with_trusted_initial_height(mut self, height: u64) -> Self {
        self.trusted_initial_height = Some(height);
        self
    }

    /// Configure directory where dumps of all requests and responses will be saved.
    /// Useful for debugging.
    ///
    /// This function will create the directory if it does not exist and save dumps of
    /// * all requests and responses - in files named `msg-*.json`
    /// * retrieved quorum public keys - in files named `quorum_pubkey-*.json`
    /// * retrieved data contracts - in files named `data_contract-*.json`
    ///
    /// These files can be used together with [MockDashPlatformSdk] to replay the requests and responses.
    /// See [MockDashPlatformSdk::load_expectations_sync()] for more information.
    ///
    /// Available only when `mocks` feature is enabled.
    #[cfg(feature = "mocks")]
    pub fn with_dump_dir(mut self, dump_dir: &Path) -> Self {
        self.dump_dir = Some(dump_dir.to_path_buf());
        self
    }

    /// Build the Sdk instance.
    ///
    /// This method will create the Sdk instance based on the configuration provided to the builder.
    ///
    /// # Errors
    ///
    /// This method will return an error if the Sdk cannot be created.
    pub fn build(self) -> Result<Sdk, Error> {
        let is_network_sdk = self.addresses.is_some();
        let has_height_anchor = self
            .trusted_initial_height
            .zip(self.metadata_height_tolerance)
            .is_some_and(|(height, tolerance)| height > tolerance);
        if is_network_sdk
            && self.proofs
            && self.metadata_time_tolerance_ms.is_none()
            && !has_height_anchor
        {
            return Err(Error::Config(
                "proof mode requires a trusted initial height or signed-time freshness policy"
                    .to_string(),
            ));
        }

        let dapi_client_settings = match self.settings {
            Some(settings) => DEFAULT_REQUEST_SETTINGS.override_by(settings),
            None => DEFAULT_REQUEST_SETTINGS,
        };

        let initial_version = self.version.unwrap_or_else(|| {
            PlatformVersion::get(min_protocol_version(self.network))
                .expect("min_protocol_version for a network must be a valid version")
        });

        let sdk= match self.addresses {
            // non-mock mode
            Some(addresses) => {
                #[allow(unused_mut)] // needs to be mutable for features other than wasm
                let mut dapi = DapiClient::new(addresses, dapi_client_settings);
                #[cfg(not(target_arch = "wasm32"))]
                if let Some(pem) = self.ca_certificate {
                    dapi = dapi.with_ca_certificate(pem);
                }

                #[cfg(feature = "mocks")]
                let dapi = dapi.dump_dir(self.dump_dir.clone());

                #[allow(unused_mut)] // needs to be mutable for #[cfg(feature = "mocks")]
                let mut sdk= Sdk{
                    network: self.network,
                    dapi_client_settings,
                    inner:SdkInstance::Dapi { dapi },
                    proofs:self.proofs,
                    context_provider: ArcSwapOption::new( self.context_provider.map(Arc::new)),
                    cancel_token: self.cancel_token,
                    nonce_cache: Default::default(),
                    // Seed atomic with the initial version; whether the version is
                    // pinned is controlled separately by `version_pinned`.
                    protocol_version: Arc::new(atomic::AtomicU32::new(initial_version.protocol_version)),
                    version_pinned: self.version_pinned,
                    metadata_last_seen_height: Arc::new(atomic::AtomicU64::new(
                        self.trusted_initial_height.unwrap_or(0),
                    )),
                    metadata_height_tolerance: self.metadata_height_tolerance,
                    metadata_time_tolerance_ms: self.metadata_time_tolerance_ms,
                    #[cfg(feature = "mocks")]
                    dump_dir: self.dump_dir,
                };
                // if context provider is not set correctly (is None), it means we need to fall back to core wallet
                if  sdk.context_provider.load().is_none() {
                    #[cfg(feature = "mocks")]
                    if !self.core_ip.is_empty() {
                        tracing::warn!(
                            "ContextProvider not set, falling back to a mock one; use SdkBuilder::with_context_provider() to set it up");
                        let mut context_provider = GrpcContextProvider::new(None,
                            &self.core_ip, self.core_port, &self.core_user, &self.core_password,
                            self.data_contract_cache_size, self.token_config_cache_size, self.quorum_public_keys_cache_size)?;
                        #[cfg(feature = "mocks")]
                        if sdk.dump_dir.is_some() {
                            context_provider.set_dump_dir(sdk.dump_dir.clone());
                        }
                        // We have cyclical dependency Sdk <-> GrpcContextProvider, so we just do some
                        // workaround using additional Arc.
                        let context_provider= Arc::new(context_provider);
                        sdk.context_provider.swap(Some(Arc::new(Box::new(context_provider.clone()))));
                        context_provider.set_sdk(Some(sdk.clone()));
                    } else{
                        return Err(Error::Config(concat!(
                            "context provider is not set, configure it with SdkBuilder::with_context_provider() ",
                            "or configure Core access with SdkBuilder::with_core() to use mock context provider")
                            .to_string()));
                    }
                    #[cfg(not(feature = "mocks"))]
                    return Err(Error::Config(concat!(
                        "context provider is not set, configure it with SdkBuilder::with_context_provider() ",
                        "or enable `mocks` feature to use mock context provider")
                        .to_string()));
                };

                sdk
            },
            #[cfg(feature = "mocks")]
            // mock mode
            None => {
                let dapi =Arc::new(Mutex::new(  MockDapiClient::new()));
                // We create mock context provider that will use the mock DAPI client to retrieve data contracts.
                let  context_provider = self.context_provider.unwrap_or_else(||{
                    let mut cp=MockContextProvider::new();
                    if let Some(ref dump_dir) = self.dump_dir {
                        cp.quorum_keys_dir(Some(dump_dir.clone()));
                    }
                    Box::new(cp)
                }
                );
                let mock_sdk = MockDashPlatformSdk::new(Arc::clone(&dapi));
                let mock_sdk = Arc::new(Mutex::new(mock_sdk));
                let sdk= Sdk {
                    network: self.network,
                    dapi_client_settings,
                    inner:SdkInstance::Mock {
                        mock:mock_sdk.clone(),
                        dapi,
                        address_list: AddressList::new(),
                    },
                    dump_dir: self.dump_dir.clone(),
                    proofs:self.proofs,
                    nonce_cache: Default::default(),
                    protocol_version: Arc::new(atomic::AtomicU32::new(initial_version.protocol_version)),
                    version_pinned: self.version_pinned,
                    context_provider: ArcSwapOption::new(Some(Arc::new(context_provider))),
                    cancel_token: self.cancel_token,
                    metadata_last_seen_height: Arc::new(atomic::AtomicU64::new(
                        self.trusted_initial_height.unwrap_or(0),
                    )),
                    metadata_height_tolerance: self.metadata_height_tolerance,
                    metadata_time_tolerance_ms: self.metadata_time_tolerance_ms,
                };
                let mut guard = mock_sdk.try_lock().expect("mock sdk is in use by another thread and cannot be reconfigured");
                guard.set_sdk(sdk.clone());
                if let Some(ref dump_dir) = self.dump_dir {
                    guard.load_expectations_sync(dump_dir)?;
                };

                sdk
            },
            #[cfg(not(feature = "mocks"))]
            None => return Err(Error::Config("Mock mode is not available. Please enable `mocks` feature or provide address list.".to_string())),
        };

        Ok(sdk)
    }
}

pub fn prettify_proof(proof: &Proof) -> String {
    let config = bincode::config::standard()
        .with_big_endian()
        .with_no_limit();
    let grovedb_proof: Result<GroveDBProof, DecodeError> =
        bincode::decode_from_slice(&proof.grovedb_proof, config).map(|(a, _)| a);

    let grovedb_proof_string = match grovedb_proof {
        Ok(proof) => format!("{}", proof),
        Err(_) => "Invalid GroveDBProof".to_string(),
    };
    format!(
        "Proof {{
            grovedb_proof: {},
            quorum_hash: 0x{},
            signature: 0x{},
            round: {},
            block_id_hash: 0x{},
            quorum_type: {},
        }}",
        grovedb_proof_string,
        hex::encode(&proof.quorum_hash),
        hex::encode(&proof.signature),
        proof.round,
        hex::encode(&proof.block_id_hash),
        proof.quorum_type,
    )
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use dapi_grpc::platform::v0::{GetIdentityRequest, ResponseMetadata};
    use rs_dapi_client::transport::TransportRequest;
    use test_case::test_matrix;

    use crate::SdkBuilder;

    use super::{min_protocol_version, Network};

    /// Mainnet Evo masternodes expose the Platform HTTP endpoint on 443.
    const MAINNET_PLATFORM_HTTP_PORT: u16 = 443;
    /// Testnet Evo masternodes expose the Platform HTTP endpoint on 1443.
    const TESTNET_PLATFORM_HTTP_PORT: u16 = 1443;

    #[test]
    fn new_testnet_sources_bootstrap_from_seeds() {
        let builder = SdkBuilder::new_testnet();
        let address_list = builder
            .addresses
            .as_ref()
            .expect("testnet builder should configure default addresses");

        assert_eq!(builder.network, Network::Testnet);
        assert!(
            !address_list.is_empty(),
            "testnet must have at least one bootstrap address"
        );
        for address in address_list.get_live_addresses() {
            assert_eq!(
                address.uri().port_u16(),
                Some(TESTNET_PLATFORM_HTTP_PORT),
                "testnet bootstrap address must use the platform HTTP port",
            );
        }
    }

    #[test]
    fn new_mainnet_sources_bootstrap_from_seeds() {
        let builder = SdkBuilder::new_mainnet();
        let address_list = builder
            .addresses
            .as_ref()
            .expect("mainnet builder should configure default addresses");

        assert_eq!(builder.network, Network::Mainnet);
        assert!(
            !address_list.is_empty(),
            "mainnet must have at least one bootstrap address"
        );
        for address in address_list.get_live_addresses() {
            assert_eq!(
                address.uri().port_u16(),
                Some(MAINNET_PLATFORM_HTTP_PORT),
                "mainnet bootstrap address must use the platform HTTP port",
            );
        }
    }

    mod seed_tls_filter {
        use super::super::{address_list_from_seeds, seed_tls_deterministically_bad};
        use dash_network_seeds::{
            CoreStatus, MasternodeSeed, MasternodeType, PlatformStatus, Reachability, SslStatus,
        };

        /// `host` disambiguates seeds — [`AddressList`] dedupes by URI, so
        /// every test seed needs a distinct IP.
        fn seed(host: u8, platform: Option<PlatformStatus>) -> MasternodeSeed {
            MasternodeSeed {
                address: format!("203.0.113.{host}:9999").parse().unwrap(),
                mn_type: MasternodeType::Evo,
                platform_http_port: Some(443),
                core: CoreStatus::default(),
                platform,
            }
        }

        fn status(ssl: SslStatus, reachable: Reachability) -> PlatformStatus {
            PlatformStatus {
                reachable,
                ssl,
                ..PlatformStatus::default()
            }
        }

        /// Every `SslStatus` × probe-reachability combination, against the
        /// contract: cert-level verdicts (`Expired`/`SelfSigned`/`Untrusted`)
        /// are deterministic regardless of reachability; `NoHandshake` is
        /// deterministic only when the probe's TCP connect succeeded;
        /// `Valid`/`Unknown`/unprobed are never rejected.
        #[test]
        fn classification_covers_every_status_combination() {
            let reachabilities = [
                Reachability::Unknown,
                Reachability::Ok,
                Reachability::Timeout,
                Reachability::Refused,
                Reachability::Error,
            ];
            for reachable in reachabilities {
                for ssl in [
                    SslStatus::Expired,
                    SslStatus::SelfSigned,
                    SslStatus::Untrusted,
                ] {
                    assert!(
                        seed_tls_deterministically_bad(Some(&status(ssl, reachable))),
                        "{ssl:?} must be rejected regardless of {reachable:?}"
                    );
                }
                for ssl in [SslStatus::Valid, SslStatus::Unknown] {
                    assert!(
                        !seed_tls_deterministically_bad(Some(&status(ssl, reachable))),
                        "{ssl:?} must never be rejected ({reachable:?})"
                    );
                }
                assert_eq!(
                    seed_tls_deterministically_bad(Some(&status(
                        SslStatus::NoHandshake,
                        reachable
                    ))),
                    reachable == Reachability::Ok,
                    "NoHandshake must be rejected only when TCP connect succeeded ({reachable:?})"
                );
            }
            assert!(
                !seed_tls_deterministically_bad(None),
                "an unprobed seed must never be rejected"
            );
        }

        #[test]
        fn filter_drops_only_deterministic_failures() {
            let seeds = vec![
                seed(1, Some(status(SslStatus::Valid, Reachability::Ok))),
                seed(2, Some(status(SslStatus::Expired, Reachability::Ok))),
                seed(
                    3,
                    Some(status(SslStatus::NoHandshake, Reachability::Timeout)),
                ),
                seed(4, Some(status(SslStatus::NoHandshake, Reachability::Ok))),
                seed(5, None),
            ];
            assert_eq!(address_list_from_seeds(&seeds, true).len(), 3);
            assert_eq!(address_list_from_seeds(&seeds, false).len(), 5);
        }

        /// The all-rejected input exercises the empty-filter result the
        /// caller falls back from; the fallback itself must retain the
        /// full set.
        #[test]
        fn all_rejected_input_yields_empty_filtered_and_full_unfiltered() {
            let seeds = vec![
                seed(1, Some(status(SslStatus::Expired, Reachability::Ok))),
                seed(2, Some(status(SslStatus::Untrusted, Reachability::Timeout))),
            ];
            assert!(address_list_from_seeds(&seeds, true).is_empty());
            assert_eq!(address_list_from_seeds(&seeds, false).len(), 2);
        }

        #[test]
        fn seed_without_platform_port_is_always_skipped() {
            let mut no_port = seed(1, Some(status(SslStatus::Valid, Reachability::Ok)));
            no_port.platform_http_port = None;
            assert!(address_list_from_seeds(&[no_port], false).is_empty());
        }
    }

    /// Smoke signal: the upstream seed lists are far larger than 10 entries on
    /// both networks. If parsing drops most of them we want a loud test
    /// failure rather than silently shipping a near-empty bootstrap list.
    #[test]
    fn bootstrap_counts_reasonable() {
        let mainnet = SdkBuilder::new_mainnet()
            .addresses
            .expect("mainnet builder should configure default addresses");
        let testnet = SdkBuilder::new_testnet()
            .addresses
            .expect("testnet builder should configure default addresses");
        assert!(
            mainnet.len() >= 10,
            "expected >=10 mainnet bootstrap addresses, got {}",
            mainnet.len()
        );
        assert!(
            testnet.len() >= 10,
            "expected >=10 testnet bootstrap addresses, got {}",
            testnet.len()
        );
    }

    #[test]
    fn network_builders_enable_an_independent_time_anchor() {
        assert_eq!(
            SdkBuilder::new_testnet().metadata_time_tolerance_ms,
            Some(super::DEFAULT_METADATA_TIME_TOLERANCE_MS)
        );
        assert_eq!(SdkBuilder::new_mock().metadata_time_tolerance_ms, None);
    }

    #[test]
    fn proof_enabled_network_builder_rejects_missing_freshness_anchor() {
        let error = SdkBuilder::new(super::AddressList::new())
            .with_time_tolerance(None)
            .build()
            .expect_err("network proof mode must have an independent freshness anchor");

        assert!(
            matches!(error, crate::Error::Config(message) if message.contains("trusted initial height"))
        );
    }

    #[test_matrix(0, 0; "zero height")]
    #[test_matrix(1, 1; "height equals tolerance")]
    #[test_matrix(1, 2; "height below tolerance")]
    fn proof_enabled_network_builder_rejects_ineffective_height_anchor(
        trusted_height: u64,
        tolerance: u64,
    ) {
        let error = SdkBuilder::new(super::AddressList::new())
            .with_time_tolerance(None)
            .with_height_tolerance(Some(tolerance))
            .with_trusted_initial_height(trusted_height)
            .build()
            .expect_err("trusted height must impose a freshness floor");

        assert!(
            matches!(error, crate::Error::Config(message) if message.contains("trusted initial height"))
        );
    }

    #[test]
    fn height_only_address_checkpoint_uses_trusted_height_floor() {
        let sdk = SdkBuilder::new_mock()
            .with_time_tolerance(None)
            .with_height_tolerance(Some(2))
            .with_trusted_initial_height(100)
            .build()
            .expect("effective trusted height should permit height-only proof mode");

        assert!(matches!(
            sdk.verify_response_metadata(
                "get_addresses_trunk_state",
                &ResponseMetadata {
                    height: 97,
                    ..Default::default()
                },
            ),
            Err(crate::Error::StaleNode(
                super::StaleNodeError::Height { .. }
            ))
        ));
        assert_eq!(
            sdk.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Acquire),
            100,
            "a rejected stale checkpoint must not lower the trusted floor"
        );
    }

    #[test]
    fn trusted_initial_height_seeds_the_high_water_mark() {
        let sdk = SdkBuilder::new_mock()
            .with_trusted_initial_height(42)
            .build()
            .expect("mock SDK should build");

        assert_eq!(
            sdk.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Acquire),
            42
        );
    }

    #[test_matrix(97..102, 100, 2, false; "valid height")]
    #[test_case(103, 100, 2, true; "invalid height")]
    fn test_verify_metadata_height(
        expected_height: u64,
        received_height: u64,
        tolerance: u64,
        expect_err: bool,
    ) {
        let metadata = ResponseMetadata {
            height: received_height,
            ..Default::default()
        };

        let last_seen_height = Arc::new(std::sync::atomic::AtomicU64::new(expected_height));

        let result =
            super::verify_metadata_height(&metadata, tolerance, Arc::clone(&last_seen_height));

        assert_eq!(result.is_err(), expect_err);
        if result.is_ok() {
            assert_eq!(
                last_seen_height.load(std::sync::atomic::Ordering::Relaxed),
                expected_height.max(received_height),
                "height high-water mark must never decrease"
            );
        }
    }

    #[test]
    fn accepted_height_tolerance_cannot_walk_the_watermark_backwards() {
        let last_seen_height = Arc::new(std::sync::atomic::AtomicU64::new(100));

        super::verify_metadata_height(
            &ResponseMetadata {
                height: 99,
                ..Default::default()
            },
            1,
            Arc::clone(&last_seen_height),
        )
        .expect("one block behind is within tolerance");
        assert_eq!(
            last_seen_height.load(std::sync::atomic::Ordering::Acquire),
            100
        );

        super::verify_metadata_height(
            &ResponseMetadata {
                height: 98,
                ..Default::default()
            },
            1,
            Arc::clone(&last_seen_height),
        )
        .expect_err("a second rollback step must be compared with the high-water mark");
        assert_eq!(
            last_seen_height.load(std::sync::atomic::Ordering::Acquire),
            100
        );

        super::verify_metadata_height(
            &ResponseMetadata {
                height: 101,
                ..Default::default()
            },
            1,
            Arc::clone(&last_seen_height),
        )
        .expect("a newer height should advance the high-water mark");
        assert_eq!(
            last_seen_height.load(std::sync::atomic::Ordering::Acquire),
            101
        );
    }

    #[test]
    fn cloned_sdk_verify_metadata_height() {
        let sdk1 = SdkBuilder::new_mock()
            .build()
            .expect("mock Sdk should be created");

        // First message verified, height 1.
        let metadata = ResponseMetadata {
            height: 1,
            ..Default::default()
        };

        // use dummy request type to satisfy generic parameter
        let request = GetIdentityRequest::default();
        sdk1.verify_response_metadata(request.method_name(), &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk1.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Relaxed),
            metadata.height,
            "initial height"
        );

        // now, we clone sdk and do two requests.
        let sdk2 = sdk1.clone();
        let sdk3 = sdk1.clone();

        // Second message verified, height 2.
        let metadata = ResponseMetadata {
            height: 2,
            ..Default::default()
        };
        // use dummy request type to satisfy generic parameter
        let request = GetIdentityRequest::default();
        sdk2.verify_response_metadata(request.method_name(), &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk1.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Relaxed),
            metadata.height,
            "first sdk should see height from second sdk"
        );
        assert_eq!(
            sdk3.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Relaxed),
            metadata.height,
            "third sdk should see height from second sdk"
        );

        // Third message verified, height 3.
        let metadata = ResponseMetadata {
            height: 3,
            ..Default::default()
        };
        // use dummy request type to satisfy generic parameter
        let request = GetIdentityRequest::default();
        sdk3.verify_response_metadata(request.method_name(), &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk1.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Relaxed),
            metadata.height,
            "first sdk should see height from third sdk"
        );

        assert_eq!(
            sdk2.metadata_last_seen_height
                .load(std::sync::atomic::Ordering::Relaxed),
            metadata.height,
            "second sdk should see height from third sdk"
        );

        // Now, using sdk1 for height 1 again should fail, as we are already at 3, with default tolerance 1.
        let metadata = ResponseMetadata {
            height: 1,
            ..Default::default()
        };

        let request = GetIdentityRequest::default();
        sdk1.verify_response_metadata(request.method_name(), &metadata)
            .expect_err("metadata should be invalid");
    }

    /// Helper: build a mock SDK with auto-detect enabled and a specific starting version.
    /// Does NOT call `with_version()` (which would disable auto-detect).
    fn mock_sdk_with_auto_detect(starting_version: u32) -> super::Sdk {
        use std::sync::atomic::Ordering;

        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("mock Sdk should be created");
        sdk.protocol_version
            .store(starting_version, Ordering::Relaxed);
        sdk
    }

    #[test]
    fn test_version_update_from_metadata() {
        let sdk = mock_sdk_with_auto_detect(1);

        assert_eq!(sdk.protocol_version_number(), 1);

        let metadata = ResponseMetadata {
            protocol_version: 2,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(sdk.protocol_version_number(), 2);
        assert_eq!(sdk.version().protocol_version, 2);
    }

    #[test]
    fn test_unknown_version_ignored() {
        use dpp::version::PlatformVersion;

        let sdk = mock_sdk_with_auto_detect(PlatformVersion::latest().protocol_version);
        let original_version = sdk.protocol_version_number();

        let metadata = ResponseMetadata {
            protocol_version: 999,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(sdk.protocol_version_number(), original_version);
        assert_eq!(sdk.version().protocol_version, original_version);
    }

    #[test]
    fn test_version_shared_between_clones() {
        let sdk = mock_sdk_with_auto_detect(1);

        let clone = sdk.clone();

        let metadata = ResponseMetadata {
            protocol_version: 2,
            height: 1,
            ..Default::default()
        };

        clone
            .verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk.protocol_version_number(),
            2,
            "original should see update from clone"
        );
    }

    #[test]
    fn test_version_downgrade_ignored() {
        let sdk = mock_sdk_with_auto_detect(2);

        assert_eq!(sdk.protocol_version_number(), 2);

        let metadata = ResponseMetadata {
            protocol_version: 1,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(sdk.protocol_version_number(), 2);
    }

    #[test]
    fn test_version_zero_ignored() {
        use dpp::version::PlatformVersion;

        let sdk = mock_sdk_with_auto_detect(PlatformVersion::latest().protocol_version);
        let original_version = sdk.protocol_version_number();

        let metadata = ResponseMetadata {
            protocol_version: 0,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(sdk.protocol_version_number(), original_version);
    }

    #[test]
    fn test_concurrent_updates_converge_to_highest() {
        use std::thread;

        let sdk = mock_sdk_with_auto_detect(1);

        assert_eq!(sdk.protocol_version_number(), 1);

        let mut handles = Vec::new();
        // Spawn threads that race to update to version 2 and version 3
        for version in [2u32, 3, 2, 3, 2, 3] {
            let sdk_clone = sdk.clone();
            handles.push(thread::spawn(move || {
                let metadata = ResponseMetadata {
                    protocol_version: version,
                    height: 1,
                    ..Default::default()
                };
                sdk_clone
                    .verify_response_metadata("test", &metadata)
                    .expect("metadata should be valid");
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        // Highest known version (3) must win regardless of thread ordering
        assert_eq!(
            sdk.protocol_version_number(),
            3,
            "concurrent updates must converge to highest version"
        );
    }

    // TC-7 (global DPP version sync) removed — set_current() is no longer called
    // from the SDK. Version is stored per-instance, not in the process-wide global.

    #[test]
    fn test_explicit_version_disables_auto_detect() {
        use dpp::version::PlatformVersion;

        // Pin at the mainnet default version. The network reporting a newer
        // version must still be ignored, because the pin disables auto-detect.
        let pinned = PlatformVersion::get(min_protocol_version(Network::Mainnet))
            .expect("mainnet-floor PV exists");
        let sdk = SdkBuilder::new_mock()
            .with_version(pinned)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(sdk.protocol_version_number(), pinned.protocol_version);
        assert!(sdk.version_pinned);

        // Network reports version 12 (> pinned) — should be ignored because version is pinned
        let metadata = ResponseMetadata {
            protocol_version: dpp::version::v12::PROTOCOL_VERSION_12,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk.protocol_version_number(),
            pinned.protocol_version,
            "pinned version must not be auto-updated"
        );
    }

    #[test]
    fn test_with_initial_version_seeds_to_older_network_version() {
        use dpp::version::PlatformVersion;

        // Caller seeds the auto-detect atomic at the mainnet default version.
        // `version_pinned` stays false, so fetch_max can still ratchet upward
        // when the network later moves to a newer PV.
        let floor = min_protocol_version(Network::Mainnet);
        let initial = PlatformVersion::get(floor).expect("mainnet-floor PV exists");
        let sdk = SdkBuilder::new_mock()
            .with_initial_version(initial)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(
            sdk.protocol_version_number(),
            floor,
            "with_initial_version must seed the atomic without pinning"
        );
        assert_eq!(sdk.version().protocol_version, floor);
        assert!(
            !sdk.version_pinned,
            "with_initial_version must keep auto-detect enabled"
        );

        // Metadata at the floor is accepted (matches current seed, no ratchet needed).
        let metadata = ResponseMetadata {
            protocol_version: floor,
            height: 1,
            ..Default::default()
        };
        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");
        assert_eq!(sdk.protocol_version_number(), floor);

        // And a newer network version still ratchets upward.
        let newer = dpp::version::v12::PROTOCOL_VERSION_12;
        assert!(newer > floor, "ratchet target must exceed the floor");
        let metadata = ResponseMetadata {
            protocol_version: newer,
            height: 2,
            ..Default::default()
        };
        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");
        assert_eq!(sdk.protocol_version_number(), newer);
    }

    #[test]
    fn test_with_initial_version_after_with_version_restores_auto_detect() {
        use dpp::version::PlatformVersion;

        // Last-write-wins composability: a later `with_initial_version`
        // must re-enable auto-detect that an earlier `with_version`
        // disabled.
        //
        // `v_old` sits at the mainnet default version so the last-write-wins
        // effect stays observable.
        let v_latest = PlatformVersion::latest();
        let v_old = PlatformVersion::get(min_protocol_version(Network::Mainnet))
            .expect("mainnet-floor PV exists");
        assert!(
            v_old.protocol_version < v_latest.protocol_version,
            "v_old must be below latest so the later ratchet is observable"
        );

        let sdk = SdkBuilder::new_mock()
            .with_version(v_latest)
            .with_initial_version(v_old)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(
            sdk.protocol_version_number(),
            v_old.protocol_version,
            "with_initial_version must overwrite the prior with_version seed"
        );
        assert!(
            !sdk.version_pinned,
            "with_initial_version must restore auto-detect after with_version disabled it"
        );

        // Ratchet upward via metadata observation works because auto-detect is on.
        let metadata = ResponseMetadata {
            protocol_version: v_latest.protocol_version,
            height: 1,
            ..Default::default()
        };
        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");
        assert_eq!(sdk.protocol_version_number(), v_latest.protocol_version);
    }

    #[test]
    fn test_mock_version_follows_outer_sdk_atomic() {
        use dpp::version::PlatformVersion;

        // Build a mock SDK with auto-detect, seeded at the mainnet default
        // version. After a metadata-driven ratchet to a newer PV, both the outer
        // SDK's `version()` and the inner
        // `MockDashPlatformSdk::version()` must report the same value — single
        // source of truth.
        let v_old = PlatformVersion::get(min_protocol_version(Network::Mainnet))
            .expect("mainnet-floor PV exists");
        let v_new = PlatformVersion::latest();
        assert!(
            v_old.protocol_version < v_new.protocol_version,
            "v_old must be below latest so the ratchet is observable"
        );

        let mut sdk = SdkBuilder::new_mock()
            .with_initial_version(v_old)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(sdk.version().protocol_version, v_old.protocol_version);
        {
            let mock = sdk.mock();
            assert_eq!(
                mock.version().protocol_version,
                v_old.protocol_version,
                "mock version must mirror outer SDK before ratchet"
            );
        }

        let metadata = ResponseMetadata {
            protocol_version: v_new.protocol_version,
            height: 1,
            ..Default::default()
        };
        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(sdk.version().protocol_version, v_new.protocol_version);
        let mock = sdk.mock();
        assert_eq!(
            mock.version().protocol_version,
            v_new.protocol_version,
            "mock version must follow outer ratchet"
        );
    }

    #[test]
    fn test_default_builder_seeds_initial_protocol_version_floor() {
        // A default (unpinned) builder uses the mainnet network, so it must seed
        // the SDK at the mainnet `min_protocol_version` floor, not at latest().
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("mock Sdk should be created");

        let expected = min_protocol_version(Network::Mainnet);
        assert_eq!(
            sdk.protocol_version_number(),
            expected,
            "unpinned mainnet SDK must boot at the mainnet floor, not latest()"
        );
        assert_eq!(sdk.version().protocol_version, expected);
        assert!(
            !sdk.version_pinned,
            "default SDK must keep auto-detect enabled"
        );
    }

    #[test]
    fn test_default_floor_ratchets_up_but_never_down() {
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("mock Sdk should be created");
        // Default (mainnet) boot floor.
        let floor = min_protocol_version(Network::Mainnet);
        assert_eq!(sdk.protocol_version_number(), floor);

        // Ratchet to a fixed known target (PV12), not `floor + N`: stays valid as the
        // floor advances, and `maybe_update_protocol_version` only accepts known versions.
        let target = dpp::version::v12::PROTOCOL_VERSION_12;
        assert!(
            target > floor,
            "ratchet test target must exceed the floor; bump it if the floor reaches v12"
        );
        sdk.maybe_update_protocol_version(target);
        assert_eq!(
            sdk.protocol_version_number(),
            target,
            "auto-detect must ratchet upward from the floor"
        );

        // Never down: an older network version is ignored.
        sdk.maybe_update_protocol_version(floor - 1);
        assert_eq!(
            sdk.protocol_version_number(),
            target,
            "ratchet must never downgrade below the highest observed version"
        );
    }

    /// Regression guard for the verify-before-ratchet security invariant.
    ///
    /// The full tampered-*signed*-proof path isn't unit-testable here: it needs a
    /// quorum BLS signature, a context provider, and a `FromProof` verifier round-trip.
    /// Both ratchet sites run the `FromProof` verifier (structural + `verify_tenderdash_proof`)
    /// BEFORE `verify_response_metadata` → `maybe_update_protocol_version`: the query path via
    /// `parse_proof_with_metadata_and_proof`, the broadcast wait-path in `broadcast.rs` (see the
    /// guard comments at both call sites). Here we lock in the ratchet's own gates: it must NOT
    /// raise the stored version off untrustworthy inputs (unknown / zero / lower), so even a
    /// metadata value that slipped past verification can't move the SDK to a bogus version.
    #[test]
    fn test_ratchet_rejects_unknown_and_non_upward_versions() {
        let sdk = SdkBuilder::new_mock()
            .build()
            .expect("mock Sdk should be created");
        // Default (mainnet) boot floor.
        let floor = min_protocol_version(Network::Mainnet);
        assert_eq!(sdk.protocol_version_number(), floor);

        // Unknown (above LATEST_VERSION): rejected, version unchanged.
        sdk.maybe_update_protocol_version(dpp::version::LATEST_VERSION + 1);
        assert_eq!(
            sdk.protocol_version_number(),
            floor,
            "unknown protocol version must not move the stored version"
        );

        // Zero (e.g. metadata default / stripped field): ignored.
        sdk.maybe_update_protocol_version(0);
        assert_eq!(
            sdk.protocol_version_number(),
            floor,
            "zero protocol version must be ignored"
        );

        // Equal: no-op (no spurious downgrade or churn).
        sdk.maybe_update_protocol_version(floor);
        assert_eq!(sdk.protocol_version_number(), floor);

        // Lower known version: ignored by the upward-only guard.
        sdk.maybe_update_protocol_version(floor - 1);
        assert_eq!(
            sdk.protocol_version_number(),
            floor,
            "lower known version must not downgrade the stored version"
        );
    }

    /// A pin *below* the per-network [`min_protocol_version`] is preserved as-is
    /// (no construction-time clamp) and `version_pinned` stays `true`.
    #[test]
    fn test_explicit_pin_below_floor_is_preserved() {
        use dpp::version::PlatformVersion;

        let floor = min_protocol_version(Network::Mainnet);
        let below = floor - 1;
        let pinned = PlatformVersion::get(below).expect("sub-floor PV exists");
        let sdk = SdkBuilder::new_mock()
            .with_version(pinned)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(
            sdk.protocol_version_number(),
            below,
            "a pin below the floor must be preserved"
        );
        // Still pinned: auto-detect stays disabled.
        assert!(sdk.version_pinned);
    }

    // -----------------------------------------------------------------
    // per-network protocol-version floor + non-mainnet boot/refresh
    // -----------------------------------------------------------------

    /// An unpinned testnet SDK boots at the `min_protocol_version` floor, just
    /// like the mainnet default, and stays there until a proven response ratchets
    /// it upward.
    #[test]
    fn test_testnet_default_builder_boots_at_per_network_floor() {
        let sdk = SdkBuilder::new_mock()
            .with_network(Network::Testnet)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(
            sdk.protocol_version_number(),
            min_protocol_version(Network::Testnet),
            "testnet seeds directly at its per-network floor"
        );
        assert!(!sdk.version_pinned);
    }

    #[test_matrix([90,91,100,109,110], 100, 10, false; "valid time")]
    #[test_matrix([0,89,111], 100, 10, true; "invalid time")]
    #[test_matrix([0,100], [0,100], 100, false; "zero time")]
    #[test_matrix([99,101], 100, 0, true; "zero tolerance")]
    fn test_verify_metadata_time(
        received_time: u64,
        now_time: u64,
        tolerance: u64,
        expect_err: bool,
    ) {
        let metadata = ResponseMetadata {
            time_ms: received_time,
            ..Default::default()
        };

        let result = super::verify_metadata_time(&metadata, now_time, tolerance);

        assert_eq!(result.is_err(), expect_err);
    }

    // -----------------------------------------------------------------
    // refresh_protocol_version
    // -----------------------------------------------------------------

    /// Register a proven `ExtendedEpochInfo::fetch_current` expectation on the
    /// mock SDK. The mock injects `LATEST_VERSION` into the proven response's
    /// metadata, so consuming this expectation drives `refresh_protocol_version`
    /// through the same verified `maybe_update_protocol_version` ratchet a real
    /// quorum-signed response would — the exact path production relies on.
    async fn expect_epoch_refresh(sdk: &mut super::Sdk) {
        use crate::platform::types::epoch::EpochQuery;
        use crate::platform::LimitQuery;
        use dpp::block::extended_epoch_info::{v0::ExtendedEpochInfoV0, ExtendedEpochInfo};
        use drive_proof_verifier::types::ExtendedEpochInfos;

        // Must match the two queries `ExtendedEpochInfo::fetch_current` issues: a
        // genesis probe, then a two-epoch ascending confirmation from the hinted
        // current epoch (mock expectation metadata reports epoch 0, so the hint is
        // 0). The confirmation answers with epoch 0 alone, which is how a real
        // proof says "no epoch above 0 has started".
        let probe_query = LimitQuery {
            query: EpochQuery::genesis(),
            limit: Some(1),
            start_info: None,
        };
        let confirmation_query = LimitQuery {
            query: EpochQuery::ascending_from(0),
            limit: Some(2),
            start_info: None,
        };

        let epoch = ExtendedEpochInfo::from(ExtendedEpochInfoV0 {
            index: 0,
            first_block_time: 0,
            first_block_height: 0,
            first_core_block_height: 0,
            fee_multiplier_permille: 0,
            protocol_version: dpp::version::LATEST_VERSION,
        });

        sdk.mock()
            .expect_fetch::<ExtendedEpochInfo, _>(probe_query, Some(epoch.clone()))
            .await
            .expect("register epoch probe expectation");
        sdk.mock()
            .expect_fetch_many::<_, ExtendedEpochInfo, _, ExtendedEpochInfos>(
                confirmation_query,
                Some(ExtendedEpochInfos::from_iter([(0, Some(epoch))])),
            )
            .await
            .expect("register epoch refresh expectation");
    }

    /// Seeded below `LATEST_VERSION`, a proven refresh ratchets the SDK up to the
    /// network's version through the *verified* metadata path (the mock injects
    /// `LATEST_VERSION` into the proven response's metadata, exactly as a real
    /// quorum-signed response would). Mirrors the testnet shielded-fee
    /// under-reservation regression.
    #[tokio::test]
    async fn test_refresh_ratchets_up_via_proven_query() {
        let mut sdk = mock_sdk_with_auto_detect(super::min_protocol_version(Network::Mainnet));
        assert_eq!(
            sdk.protocol_version_number(),
            super::min_protocol_version(Network::Mainnet)
        );

        expect_epoch_refresh(&mut sdk).await;

        let resulting = sdk
            .refresh_protocol_version()
            .await
            .expect("refresh should succeed");

        assert_eq!(
            resulting,
            dpp::version::LATEST_VERSION,
            "returned version must reflect the ratchet to the network's latest"
        );
        assert_eq!(sdk.protocol_version_number(), dpp::version::LATEST_VERSION);
        assert_eq!(sdk.version().protocol_version, dpp::version::LATEST_VERSION);
    }

    /// A pinned (explicit `with_version`) SDK has opted out of version tracking:
    /// `refresh_protocol_version` short-circuits to a no-op that returns the
    /// pinned version without issuing any network request — so it succeeds even
    /// with no mock expectation registered.
    #[tokio::test]
    async fn test_refresh_leaves_pinned_sdk_unchanged() {
        use dpp::version::PlatformVersion;

        // Pin at the mainnet default version.
        let pinned = PlatformVersion::get(min_protocol_version(Network::Mainnet))
            .expect("mainnet-floor PV exists");
        let sdk = SdkBuilder::new_mock()
            .with_version(pinned)
            .build()
            .expect("mock Sdk should be created");
        assert_eq!(sdk.protocol_version_number(), pinned.protocol_version);
        assert!(sdk.version_pinned);

        // No expectation registered: a pinned refresh must not even attempt the
        // query, so this returns Ok with the pinned version unchanged.
        let resulting = sdk
            .refresh_protocol_version()
            .await
            .expect("pinned refresh is a no-op and must not error");

        assert_eq!(
            resulting, pinned.protocol_version,
            "pinned version must not move"
        );
        assert_eq!(sdk.protocol_version_number(), pinned.protocol_version);
    }

    /// When the proven query is unavailable (no mock expectation, so the fetch
    /// errors), refresh is non-fatal and does *not* fall back to an unverified
    /// version: it leaves the stored version exactly where it was. There is no
    /// runtime clamp — the auto-detect ratchet only ever moves it upward.
    #[tokio::test]
    async fn test_refresh_query_unavailable_keeps_current_version() {
        let starting = min_protocol_version(Network::Mainnet);
        let sdk = mock_sdk_with_auto_detect(starting);
        assert_eq!(sdk.protocol_version_number(), starting);

        let resulting = sdk
            .refresh_protocol_version()
            .await
            .expect("refresh is best-effort and must not error when the query fails");

        assert_eq!(
            resulting, starting,
            "a failed refresh must leave the stored version untouched (no fallback)"
        );
        assert_eq!(sdk.protocol_version_number(), starting);
    }
}
