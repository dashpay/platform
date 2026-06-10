//! [Sdk] entrypoint to Dash Platform.

use crate::error::{Error, StaleNodeError};
use crate::internal_cache::NonceCache;
use crate::mock::MockResponse;
#[cfg(feature = "mocks")]
use crate::mock::{provider::GrpcContextProvider, MockDashPlatformSdk};
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
use dpp::dashcore::Network;
use dpp::prelude::IdentityNonce;
use dpp::version::PlatformVersion;
use drive::grovedb::operations::proof::GroveDBProof;
use drive_proof_verifier::FromProof;
pub use http::Uri;
#[cfg(feature = "mocks")]
use rs_dapi_client::mock::MockDapiClient;
pub use rs_dapi_client::Address;
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
/// The default metadata time tolerance for checkpoint queries in milliseconds
const ADDRESS_STATE_TIME_TOLERANCE_MS: u64 = 31 * 60 * 1000;

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
/// ## Panics
///
/// Panics on networks other than `Mainnet` and `Testnet` — no upstream
/// seed list exists for devnet/regtest.
fn default_address_list_for_network(network: Network) -> AddressList {
    if !matches!(network, Network::Mainnet | Network::Testnet) {
        panic!("default address list is only available for mainnet and testnet");
    }
    let mut list = AddressList::new();
    for seed in dash_network_seeds::evo_seeds(network) {
        let Some(port) = seed.platform_http_port else {
            continue;
        };
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

    /// Whether to auto-detect protocol version from network response metadata.
    /// Set to `false` when the user explicitly calls [`SdkBuilder::with_version()`].
    auto_detect_protocol_version: bool,

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
            auto_detect_protocol_version: self.auto_detect_protocol_version,
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
                None,
                self.metadata_time_tolerance_ms
                    .and(Some(ADDRESS_STATE_TIME_TOLERANCE_MS)),
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
        if let Some(height_tolerance) = metadata_height_tolerance {
            verify_metadata_height(
                metadata,
                height_tolerance,
                Arc::clone(&(self.metadata_last_seen_height)),
            )?;
        };
        if let Some(time_tolerance) = metadata_time_tolerance_ms {
            let now = chrono::Utc::now().timestamp_millis() as u64;
            verify_metadata_time(metadata, now, time_tolerance)?;
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
        if !self.auto_detect_protocol_version {
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

    // TODO: Changed to public for tests
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
    /// first call to this method uses [`PlatformVersion::latest()`] as a fallback because no
    /// network response has been received yet to teach the SDK the real network version.
    ///
    /// The actual network version is learned only *after* proof parsing succeeds, when
    /// [`Self::verify_response_metadata()`] processes `metadata.protocol_version`.  If the
    /// connected network runs an older protocol version **and** proof interpretation differs
    /// between that version and `latest()`, the very first request may fail before the SDK can
    /// correct itself.  Subsequent requests will use the correct version.
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
    /// When auto-detection is enabled (default), returns [`PlatformVersion::latest()`]
    /// until the first network response is received, then tracks the network's version.
    /// When pinned via [`SdkBuilder::with_version()`], always returns the pinned version.
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
    let mut expected_height = last_seen_height.load(Ordering::Relaxed);
    let received_height = metadata.height;

    // Same height, no need to update.
    if received_height == expected_height {
        tracing::trace!(
            expected_height,
            received_height,
            tolerance,
            "received message has the same height as previously seen"
        );
        return Ok(());
    }

    // If expected_height <= tolerance, then Sdk just started, so we just assume what we got is correct.
    if expected_height > tolerance && received_height < expected_height - tolerance {
        return Err(StaleNodeError::Height {
            expected_height,
            received_height,
            tolerance_blocks: tolerance,
        }
        .into());
    }

    // New height is ahead of the last seen height, so we update the last seen height.
    tracing::trace!(
        expected_height = expected_height,
        received_height = received_height,
        tolerance,
        "received message with new height"
    );
    while let Err(stored_height) = last_seen_height.compare_exchange(
        expected_height,
        received_height,
        Ordering::SeqCst,
        Ordering::Relaxed,
    ) {
        // The value was changed to a higher value by another thread, so we need to retry.
        if stored_height >= metadata.height {
            break;
        }
        expected_height = stored_height;
    }

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

    /// Platform version to use in this Sdk
    version: &'static PlatformVersion,

    /// Whether the user explicitly called `with_version()`.
    /// When true, auto-detection of protocol version from network metadata is disabled.
    version_explicit: bool,

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

            version: PlatformVersion::latest(),
            version_explicit: false,
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
    /// Select specific version of Dash Platform to use.
    ///
    /// Defaults to [PlatformVersion::latest()].
    pub fn with_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = version;
        self.version_explicit = true;
        self
    }

    /// Set the *initial* protocol version seed for auto-detect mode.
    ///
    /// Unlike [`Self::with_version`], this leaves auto-detect active —
    /// the SDK starts at `version.protocol_version` and ratchets upward
    /// (via `fetch_max` in `maybe_update_protocol_version`) once the
    /// network's actual version is observed in response metadata.
    ///
    /// Use this when an SDK built against `PlatformVersion::latest()`
    /// must talk to a network running an older protocol version (e.g.
    /// a v3.0 testnet from a v3.1+ binary). Without an explicit initial
    /// version, the SDK's `version()` fallback returns `latest()` until
    /// the first response is parsed, and the upward-only `fetch_max`
    /// guard can never ratchet *down* to the older network — leaving
    /// any version-dispatched encoders (e.g. the documents query) to
    /// ship a too-new wire shape that the network rejects.
    ///
    /// Seeds `self.version` and resets `version_explicit` to `false`, so
    /// auto-detect is (re-)enabled. Builder chains use last-write-wins:
    /// calling `with_initial_version` after `with_version` restores
    /// auto-detect rather than silently keeping it disabled.
    ///
    /// **Caveat**: this protection only holds for encoders whose
    /// `drive_abci.query.<name>.default_current_version` is correctly pinned per
    /// historical PV. New versioned encoders must follow the same per-PV pinning
    /// pattern as `document_query`.
    pub fn with_initial_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = version;
        self.version_explicit = false;
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
    /// This is set to `None` by default.
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
        let dapi_client_settings = match self.settings {
            Some(settings) => DEFAULT_REQUEST_SETTINGS.override_by(settings),
            None => DEFAULT_REQUEST_SETTINGS,
        };

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
                    // Seed atomic with self.version; whether auto-detect is on
                    // is controlled separately by `version_explicit`.
                    protocol_version: Arc::new(atomic::AtomicU32::new(
                        self.version.protocol_version,
                    )),
                    auto_detect_protocol_version: !self.version_explicit,
                    // Note: in the future, we need to securely initialize initial height during Sdk bootstrap or first request.
                    metadata_last_seen_height: Arc::new(atomic::AtomicU64::new(0)),
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
                    protocol_version: Arc::new(atomic::AtomicU32::new(
                        self.version.protocol_version,
                    )),
                    auto_detect_protocol_version: !self.version_explicit,
                    context_provider: ArcSwapOption::new(Some(Arc::new(context_provider))),
                    cancel_token: self.cancel_token,
                    metadata_last_seen_height: Arc::new(atomic::AtomicU64::new(0)),
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

    use super::Network;

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
                received_height,
                "previous height should be updated"
            );
        }
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

        // Explicitly pin to version 1 via with_version()
        let sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::get(1).unwrap())
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(sdk.protocol_version_number(), 1);
        assert!(!sdk.auto_detect_protocol_version);

        // Network reports version 2 — should be ignored because version is pinned
        let metadata = ResponseMetadata {
            protocol_version: 2,
            height: 1,
            ..Default::default()
        };

        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");

        assert_eq!(
            sdk.protocol_version_number(),
            1,
            "pinned version must not be auto-updated"
        );
    }

    #[test]
    fn test_with_initial_version_seeds_to_older_network_version() {
        use dpp::version::PlatformVersion;

        // Caller knows the network is on PV 1 and seeds the auto-detect
        // atomic accordingly. `version_explicit` stays false, so fetch_max
        // can still ratchet upward when the network later moves to a newer PV.
        let initial = PlatformVersion::get(1).expect("PV 1 exists");
        let sdk = SdkBuilder::new_mock()
            .with_initial_version(initial)
            .build()
            .expect("mock Sdk should be created");

        assert_eq!(
            sdk.protocol_version_number(),
            1,
            "with_initial_version must seed the atomic without pinning"
        );
        assert_eq!(sdk.version().protocol_version, 1);

        // Metadata at PV 1 is accepted (matches current seed, no ratchet needed).
        let metadata = ResponseMetadata {
            protocol_version: 1,
            height: 1,
            ..Default::default()
        };
        sdk.verify_response_metadata("test", &metadata)
            .expect("metadata should be valid");
        assert_eq!(sdk.protocol_version_number(), 1);
    }

    #[test]
    fn test_with_initial_version_after_with_version_restores_auto_detect() {
        use dpp::version::PlatformVersion;

        // Last-write-wins composability: a later `with_initial_version`
        // must re-enable auto-detect that an earlier `with_version`
        // disabled.
        let v_latest = PlatformVersion::latest();
        let v_old = PlatformVersion::get(1).expect("PV 1 exists");

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
            sdk.auto_detect_protocol_version,
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

        // Build a mock SDK with auto-detect, seeded at PV 1. After a
        // metadata-driven ratchet to a newer PV, both the outer SDK's
        // `version()` and the inner `MockDashPlatformSdk::version()`
        // must report the same value — single source of truth.
        let v_old = PlatformVersion::get(1).expect("PV 1 exists");
        let v_new = PlatformVersion::latest();

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
            "mock version must follow outer ratchet (CMT-001 regression)"
        );
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
}
