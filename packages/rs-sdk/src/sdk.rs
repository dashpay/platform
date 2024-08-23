//! [Sdk] entrypoint to Dash Platform.

use crate::error::Error;
use crate::internal_cache::InternalSdkCache;
use crate::mock::MockResponse;
#[cfg(feature = "mocks")]
use crate::mock::{provider::GrpcContextProvider, MockDashPlatformSdk};
use crate::platform::transition::put_settings::PutSettings;
use crate::platform::{Fetch, Identifier};
use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dpp::bincode;
use dpp::bincode::error::DecodeError;
use dpp::dashcore::Network;
use dpp::identity::identity_nonce::IDENTITY_NONCE_VALUE_FILTER;
use dpp::prelude::IdentityNonce;
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive::grovedb::operations::proof::GroveDBProof;
use drive_proof_verifier::types::{IdentityContractNonceFetcher, IdentityNonceFetcher};
#[cfg(feature = "mocks")]
use drive_proof_verifier::MockContextProvider;
use drive_proof_verifier::{ContextProvider, FromProof};
pub use http::Uri;
#[cfg(feature = "mocks")]
use rs_dapi_client::mock::MockDapiClient;
pub use rs_dapi_client::AddressList;
pub use rs_dapi_client::RequestSettings;
use rs_dapi_client::{
    transport::{TransportClient, TransportRequest},
    DapiClient, DapiClientError, DapiRequestExecutor,
};
use std::collections::btree_map::Entry;
use std::fmt::Debug;
#[cfg(feature = "mocks")]
use std::num::NonZeroUsize;
#[cfg(feature = "mocks")]
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(feature = "mocks")]
use tokio::sync::{Mutex, MutexGuard};
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

/// How many data contracts fit in the cache.
pub const DEFAULT_CONTRACT_CACHE_SIZE: usize = 100;
/// How many quorum public keys fit in the cache.
pub const DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE: usize = 100;
/// The default identity nonce stale time in seconds
pub const DEFAULT_IDENTITY_NONCE_STALE_TIME_S: u64 = 1200; //20 mins

/// a type to represent staleness in seconds
pub type StalenessInSeconds = u64;

/// The last query timestamp
pub type LastQueryTimestamp = u64;

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
#[derive(Clone)]
pub struct Sdk {
    /// The network that the sdk is configured for (Dash (mainnet), Testnet, Devnet, Regtest)  
    pub network: Network,
    inner: SdkInstance,
    /// Use proofs when retrieving data from Platform.
    ///
    /// This is set to `true` by default. `false` is not implemented yet.
    proofs: bool,

    /// An internal SDK cache managed exclusively by the SDK
    internal_cache: Arc<InternalSdkCache>,

    /// Context provider used by the SDK.
    ///
    /// ## Panics
    ///
    /// Note that setting this to None can panic.
    context_provider: Option<Arc<Box<dyn ContextProvider>>>,

    /// Cancellation token; once cancelled, all pending requests should be aborted.
    pub(crate) cancel_token: CancellationToken,

    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,
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

        /// Platform version configured for this Sdk
        version: &'static PlatformVersion,
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

        /// Platform version configured for this Sdk
        version: &'static PlatformVersion,
    },
}

impl Sdk {
    /// Initialize Dash Platform  SDK in mock mode.
    ///
    /// This is a helper method that uses [`SdkBuilder`] to initialize the SDK in mock mode.
    ///
    /// See also [`SdkBuilder`].
    pub fn new_mock() -> Self {
        SdkBuilder::default()
            .build()
            .expect("mock should be created")
    }

    /// Retrieve object `O` from proof contained in `request` (of type `R`) and `response`.
    ///
    /// This method is used to retrieve objects from proofs returned by Dash Platform.
    ///
    /// ## Generic Parameters
    ///
    /// - `R`: Type of the request that was used to fetch the proof.
    /// - `O`: Type of the object to be retrieved from the proof.
    pub(crate) async fn parse_proof<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: Mockable,
    {
        self.parse_proof_with_metadata(request, response)
            .await
            .map(|result| result.0)
    }

    /// Retrieve object `O` from proof contained in `request` (of type `R`) and `response`.
    ///
    /// This method is used to retrieve objects from proofs returned by Dash Platform.
    ///
    /// ## Generic Parameters
    ///
    /// - `R`: Type of the request that was used to fetch the proof.
    /// - `O`: Type of the object to be retrieved from the proof.
    pub(crate) async fn parse_proof_with_metadata<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<(Option<O>, ResponseMetadata), drive_proof_verifier::Error>
    where
        O::Request: Mockable,
    {
        let provider = self
            .context_provider
            .as_ref()
            .ok_or(drive_proof_verifier::Error::ContextProviderNotSet)?;

        match self.inner {
            SdkInstance::Dapi { .. } => O::maybe_from_proof_with_metadata(
                request,
                response,
                self.network,
                self.version(),
                &provider,
            )
            .map(|(a, b, _)| (a, b)),
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mock, .. } => {
                let guard = mock.lock().await;
                guard
                    .parse_proof_with_metadata(request, response)
                    .map(|(a, b, _)| (a, b))
            }
        }
    }

    /// Retrieve object `O` from proof contained in `request` (of type `R`) and `response`.
    ///
    /// This method is used to retrieve objects from proofs returned by Dash Platform.
    ///
    /// ## Generic Parameters
    ///
    /// - `R`: Type of the request that was used to fetch the proof.
    /// - `O`: Type of the object to be retrieved from the proof.
    pub(crate) async fn parse_proof_with_metadata_and_proof<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<(Option<O>, ResponseMetadata, Proof), drive_proof_verifier::Error>
    where
        O::Request: Mockable,
    {
        let provider = self
            .context_provider
            .as_ref()
            .ok_or(drive_proof_verifier::Error::ContextProviderNotSet)?;

        match self.inner {
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
        }
    }
    pub fn context_provider(&self) -> Option<impl ContextProvider> {
        self.context_provider.as_ref().map(Arc::clone)
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
    pub fn mock(&mut self) -> MutexGuard<MockDashPlatformSdk> {
        if let Sdk {
            inner: SdkInstance::Mock { ref mock, .. },
            ..
        } = self
        {
            mock.try_lock()
                .expect("mock sdk is in use by another thread and connot be reconfigured")
        } else {
            panic!("not a mock")
        }
    }

    /// Updates or fetches the nonce for a given identity from the cache,
    /// querying Platform if the cached value is stale or absent. Optionally
    /// increments the nonce before storing it, based on the provided settings.
    pub async fn get_identity_nonce(
        &self,
        identity_id: Identifier,
        bump_first: bool,
        settings: Option<PutSettings>,
    ) -> Result<IdentityNonce, Error> {
        let settings = settings.unwrap_or_default();
        let current_time_s = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };

        // we start by only using a read lock, as this speeds up the system
        let mut identity_nonce_counter = self.internal_cache.identity_nonce_counter.lock().await;
        let entry = identity_nonce_counter.entry(identity_id);

        let should_query_platform = match &entry {
            Entry::Vacant(_) => true,
            Entry::Occupied(e) => {
                let (_, last_query_time) = e.get();
                *last_query_time
                    < current_time_s.saturating_sub(
                        settings
                            .identity_nonce_stale_time_s
                            .unwrap_or(DEFAULT_IDENTITY_NONCE_STALE_TIME_S),
                    )
            }
        };

        if should_query_platform {
            let platform_nonce = IdentityNonceFetcher::fetch_with_settings(
                self,
                identity_id,
                settings.request_settings,
            )
            .await?
            .unwrap_or(IdentityNonceFetcher(0))
            .0;
            match entry {
                Entry::Vacant(e) => {
                    let insert_nonce = if bump_first {
                        platform_nonce + 1
                    } else {
                        platform_nonce
                    };
                    e.insert((insert_nonce, current_time_s));
                    Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                }
                Entry::Occupied(mut e) => {
                    let (current_nonce, _) = e.get();
                    let insert_nonce = if platform_nonce > *current_nonce {
                        if bump_first {
                            platform_nonce + 1
                        } else {
                            platform_nonce
                        }
                    } else if bump_first {
                        *current_nonce + 1
                    } else {
                        *current_nonce
                    };
                    e.insert((insert_nonce, current_time_s));
                    Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                }
            }
        } else {
            match entry {
                Entry::Vacant(_) => {
                    panic!("this can not happen, vacant entry not possible");
                }
                Entry::Occupied(mut e) => {
                    let (current_nonce, _) = e.get();
                    if bump_first {
                        let insert_nonce = current_nonce + 1;
                        e.insert((insert_nonce, current_time_s));
                        Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                    } else {
                        Ok(*current_nonce & IDENTITY_NONCE_VALUE_FILTER)
                    }
                }
            }
        }
    }

    /// Updates or fetches the nonce for a given identity and contract pair from a cache,
    /// querying Platform if the cached value is stale or absent. Optionally
    /// increments the nonce before storing it, based on the provided settings.
    pub async fn get_identity_contract_nonce(
        &self,
        identity_id: Identifier,
        contract_id: Identifier,
        bump_first: bool,
        settings: Option<PutSettings>,
    ) -> Result<IdentityNonce, Error> {
        let settings = settings.unwrap_or_default();
        let current_time_s = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(n) => n.as_secs(),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        };

        // we start by only using a read lock, as this speeds up the system
        let mut identity_contract_nonce_counter = self
            .internal_cache
            .identity_contract_nonce_counter
            .lock()
            .await;
        let entry = identity_contract_nonce_counter.entry((identity_id, contract_id));

        let should_query_platform = match &entry {
            Entry::Vacant(_) => true,
            Entry::Occupied(e) => {
                let (_, last_query_time) = e.get();
                *last_query_time
                    < current_time_s.saturating_sub(
                        settings
                            .identity_nonce_stale_time_s
                            .unwrap_or(DEFAULT_IDENTITY_NONCE_STALE_TIME_S),
                    )
            }
        };

        if should_query_platform {
            let platform_nonce = IdentityContractNonceFetcher::fetch_with_settings(
                self,
                (identity_id, contract_id),
                settings.request_settings,
            )
            .await?
            .unwrap_or(IdentityContractNonceFetcher(0))
            .0;
            match entry {
                Entry::Vacant(e) => {
                    let insert_nonce = if bump_first {
                        platform_nonce + 1
                    } else {
                        platform_nonce
                    };
                    e.insert((insert_nonce, current_time_s));
                    Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                }
                Entry::Occupied(mut e) => {
                    let (current_nonce, _) = e.get();
                    let insert_nonce = if platform_nonce > *current_nonce {
                        if bump_first {
                            platform_nonce + 1
                        } else {
                            platform_nonce
                        }
                    } else if bump_first {
                        *current_nonce + 1
                    } else {
                        *current_nonce
                    };
                    e.insert((insert_nonce, current_time_s));
                    Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                }
            }
        } else {
            match entry {
                Entry::Vacant(_) => {
                    panic!("this can not happen, vacant entry not possible");
                }
                Entry::Occupied(mut e) => {
                    let (current_nonce, _) = e.get();
                    if bump_first {
                        let insert_nonce = current_nonce + 1;
                        e.insert((insert_nonce, current_time_s));
                        Ok(insert_nonce & IDENTITY_NONCE_VALUE_FILTER)
                    } else {
                        Ok(*current_nonce & IDENTITY_NONCE_VALUE_FILTER)
                    }
                }
            }
        }
    }

    /// Return [Dash Platform version](PlatformVersion) information used by this SDK.
    ///
    ///
    ///
    /// This is the version configured in [`SdkBuilder`].
    /// Useful whenever you need to provide [PlatformVersion] to other SDK and DPP methods.
    pub fn version<'v>(&self) -> &'v PlatformVersion {
        match &self.inner {
            SdkInstance::Dapi { version, .. } => version,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { version, .. } => version,
        }
    }

    /// Indicate if the sdk should request and verify proofs.
    pub fn prove(&self) -> bool {
        self.proofs
    }

    /// Set the [ContextProvider] to use.
    ///
    /// [ContextProvider] is used to access state information, like data contracts and quorum public keys.
    ///
    /// Note that this will overwrite any previous context provider.
    pub fn set_context_provider<C: ContextProvider + 'static>(&mut self, context_provider: C) {
        self.context_provider
            .replace(Arc::new(Box::new(context_provider)));
    }

    /// Returns a future that resolves when the Sdk is cancelled (eg. shutdown was requested).
    pub fn cancelled(&self) -> WaitForCancellationFuture {
        self.cancel_token.cancelled()
    }

    /// Request shutdown of the Sdk and all related operation.
    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }
}

#[async_trait::async_trait]
impl DapiRequestExecutor for Sdk {
    async fn execute<R: TransportRequest>(
        &self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>> {
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
    /// List of addressses to connect to.
    ///
    /// If `None`, a mock client will be created.
    addresses: Option<AddressList>,
    settings: RequestSettings,

    network: Network,

    core_ip: String,
    core_port: u16,
    core_user: String,
    core_password: String,

    /// If true, request and verify proofs of the responses.
    proofs: bool,

    /// Platform version to use in this Sdk
    version: &'static PlatformVersion,

    /// Cache size for data contracts. Used by mock [GrpcContextProvider].
    #[cfg(feature = "mocks")]
    data_contract_cache_size: NonZeroUsize,

    /// Cache size for quorum public keys. Used by mock [GrpcContextProvider].
    #[cfg(feature = "mocks")]
    quorum_public_keys_cache_size: NonZeroUsize,

    /// Context provider used by the SDK.
    context_provider: Option<Box<dyn ContextProvider>>,

    /// directory where dump files will be stored
    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,

    /// Cancellation token; once cancelled, all pending requests should be aborted.
    pub(crate) cancel_token: CancellationToken,
}

impl Default for SdkBuilder {
    /// Create default SdkBuilder that will create a mock client.
    fn default() -> Self {
        Self {
            addresses: None,
            settings: RequestSettings::default(),
            network: Network::Dash,
            core_ip: "".to_string(),
            core_port: 0,
            core_password: "".to_string(),
            core_user: "".to_string(),

            proofs: true,

            #[cfg(feature = "mocks")]
            data_contract_cache_size: NonZeroUsize::new(DEFAULT_CONTRACT_CACHE_SIZE)
                .expect("data conttact cache size must be positive"),
            #[cfg(feature = "mocks")]
            quorum_public_keys_cache_size: NonZeroUsize::new(DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE)
                .expect("quorum public keys cache size must be positive"),

            context_provider: None,

            cancel_token: CancellationToken::new(),

            version: PlatformVersion::latest(),

            #[cfg(feature = "mocks")]
            dump_dir: None,
        }
    }
}

impl SdkBuilder {
    /// Create a new SdkBuilder with provided address list.
    pub fn new(addresses: AddressList) -> Self {
        Self {
            addresses: Some(addresses),
            ..Default::default()
        }
    }

    /// Create a new SdkBuilder that will generate mock client.
    pub fn new_mock() -> Self {
        Self::default()
    }

    /// Create a new SdkBuilder instance preconfigured for testnet. NOT IMPLEMENTED YET.
    ///
    /// This is a helper method that preconfigures [SdkBuilder] for testnet use.
    /// Use this method if you want to connect to Dash Platform testnet during development and testing
    /// of your solution.
    pub fn new_testnet() -> Self {
        unimplemented!(
            "Testnet address list not implemented yet. Use new() and provide address list."
        )
    }

    /// Create a new SdkBuilder instance preconfigured mainnet (production network). NOT IMPLEMENTED YET.
    ///
    /// This is a helper method that preconfigures [SdkBuilder] for production use.
    /// Use this method if you want to connect to Dash Platform mainnet with production-ready product.
    pub fn new_mainnet() -> Self {
        unimplemented!(
            "Mainnet address list not implemented yet. Use new() and provide address list."
        )
    }

    /// Configure network type.
    ///
    /// Defaults to Network::Dash which is mainnet.
    pub fn with_network(mut self, network: Network) -> Self {
        self.network = network;
        self
    }

    /// Configure request settings.
    ///
    /// Tune request settings used to connect to the Dash Platform.
    ///
    /// Defaults to [RequestSettings::default()].
    ///
    /// See [`RequestSettings`] for more information.
    pub fn with_settings(mut self, settings: RequestSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Configure platform version.
    ///
    /// Select specific version of Dash Platform to use.
    ///
    /// Defaults to [PlatformVersion::latest()].
    pub fn with_version(mut self, version: &'static PlatformVersion) -> Self {
        self.version = version;
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
    /// Once that cancellation token is cancelled, all pending requests shall teriminate.
    pub fn with_cancellation_token(mut self, cancel_token: CancellationToken) -> Self {
        self.cancel_token = cancel_token;
        self
    }

    /// Use Dash Core as a wallet and context provider.
    ///
    /// This is a convenience method that configures the SDK to use Dash Core as a wallet and context provider.
    ///
    /// For more control over the configuration, use [SdkBuilder::with_wallet()] and [SdkBuilder::with_context_provider()].
    ///
    /// This is temporary implementation, intended for development purposes.
    pub fn with_core(mut self, ip: &str, port: u16, user: &str, password: &str) -> Self {
        self.core_ip = ip.to_string();
        self.core_port = port;
        self.core_user = user.to_string();
        self.core_password = password.to_string();

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
    /// See [MockDashPlatformSdk::load_expectations()] for more information.
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
        PlatformVersion::set_current(self.version);

        let sdk= match self.addresses {
            // non-mock mode
            Some(addresses) => {
                let dapi = DapiClient::new(addresses, self.settings);
                #[cfg(feature = "mocks")]
                let dapi = dapi.dump_dir(self.dump_dir.clone());

                #[allow(unused_mut)] // needs to be mutable for #[cfg(feature = "mocks")]
                let mut sdk= Sdk{
                    network: self.network,
                    inner:SdkInstance::Dapi { dapi,  version:self.version },
                    proofs:self.proofs,
                    context_provider: self.context_provider.map(Arc::new),
                    cancel_token: self.cancel_token,
                    #[cfg(feature = "mocks")]
                    dump_dir: self.dump_dir,
                    internal_cache: Default::default(),
                };
                // if context provider is not set correctly (is None), it means we need to fallback to core wallet
                if  sdk.context_provider.is_none() {
                    #[cfg(feature = "mocks")]
                    if !self.core_ip.is_empty() {
                        tracing::warn!("ContextProvider not set; mocking with Dash Core. \
                        Please provide your own ContextProvider with SdkBuilder::with_context_provider().");

                        let mut context_provider = GrpcContextProvider::new(None,
                            &self.core_ip, self.core_port, &self.core_user, &self.core_password,
                            self.data_contract_cache_size, self.quorum_public_keys_cache_size)?;
                        #[cfg(feature = "mocks")]
                        if sdk.dump_dir.is_some() {
                            context_provider.set_dump_dir(sdk.dump_dir.clone());
                        }
                        // We have cyclical dependency Sdk <-> GrpcContextProvider, so we just do some
                        // workaround using additional Arc.
                        let context_provider= Arc::new(context_provider);
                        sdk.context_provider.replace(Arc::new(Box::new(context_provider.clone())));
                        context_provider.set_sdk(Some(sdk.clone()));
                    } else{
                        tracing::warn!(
                            "Configure ContextProvider with Sdk::with_context_provider(); otherwise Sdk will fail");
                    }
                    #[cfg(not(feature = "mocks"))]
                    tracing::warn!(
                        "Configure ContextProvider with Sdk::with_context_provider(); otherwise Sdk will fail");
                };

                sdk
            },
            #[cfg(feature = "mocks")]
            // mock mode
            None => {
                let dapi =Arc::new(tokio::sync::Mutex::new(  MockDapiClient::new()));
                // We create mock context provider that will use the mock DAPI client to retrieve data contracts.
                let  context_provider = self.context_provider.unwrap_or_else(||{
                    let mut cp=MockContextProvider::new();
                    if let Some(ref dump_dir) = self.dump_dir {
                        cp.quorum_keys_dir(Some(dump_dir.clone()));
                    }
                    Box::new(cp)
                }
                );
                let mock_sdk = MockDashPlatformSdk::new(self.version, Arc::clone(&dapi));
                let mock_sdk = Arc::new(Mutex::new(mock_sdk));
                let sdk= Sdk {
                    network: self.network,
                    inner:SdkInstance::Mock {
                        mock:mock_sdk.clone(),
                        dapi,
                        version:self.version,

                    },
                    dump_dir: self.dump_dir.clone(),
                    proofs:self.proofs,
                    internal_cache: Default::default(),
                    context_provider:Some(Arc::new(context_provider)),
                    cancel_token: self.cancel_token,
                };
                let mut guard = mock_sdk.try_lock().expect("mock sdk is in use by another thread and connot be reconfigured");
                guard.set_sdk(sdk.clone());
                if let Some(ref dump_dir) = self.dump_dir {
                    pollster::block_on(   guard.load_expectations(dump_dir))?;
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
