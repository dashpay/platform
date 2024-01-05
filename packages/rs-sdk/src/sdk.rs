//! [Sdk] entrypoint to Dash Platform.

use std::{fmt::Debug, num::NonZeroUsize, ops::DerefMut};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "mocks")]
use crate::mock::MockDashPlatformSdk;

use crate::error::Error;
use crate::mock::provider::GrpcContextProvider;
use crate::mock::MockResponse;
use crate::wallet::Wallet;

use dapi_grpc::mock::Mockable;
use dpp::prelude::{DataContract, Identifier};
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive_proof_verifier::error::ContextProviderError;
use drive_proof_verifier::MockContextProvider;
#[cfg(feature = "mocks")]
use drive_proof_verifier::{ContextProvider, FromProof};
#[cfg(feature = "mocks")]
use hex::ToHex;
pub use http::Uri;
#[cfg(feature = "mocks")]
use rs_dapi_client::mock::MockDapiClient;
pub use rs_dapi_client::AddressList;
use rs_dapi_client::{
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClient, DapiClientError, RequestSettings,
};
use tokio::sync::Mutex;
use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

/// How many data contracts fit in the cache.
pub const DEFAULT_CONTRACT_CACHE_SIZE: usize = 100;
/// How many quorum public keys fit in the cache.
pub const DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE: usize = 100;

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
/// ## Examples
///
/// See tests/ for examples of using the SDK.
pub struct Sdk {
    inner: SdkInstance,
    /// Use proofs when retrieving data from the platform.
    ///
    /// This is set to `true` by default. `false` is not implemented yet.
    proofs: bool,

    /// Context provider used by the SDK.
    ///
    /// ## Panics
    ///
    /// Note that setting this to None can panic.
    context_provider: std::sync::Mutex<Option<Box<dyn ContextProvider>>>,

    /// Default wallet to use when executing various operations.
    ///
    /// Wallet is used to manage keys and sign transactions.
    /// It can be changed on runtime with set_wallet().
    ///
    /// If wallet is not provided, only read operations will be available.
    // TODO: replace tokio::sync::Mutex with another one implementing Send+Sync but working in both sync and async context
    pub(crate) wallet: tokio::sync::Mutex<Option<Box<dyn Wallet>>>,

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
#[derive(Debug)]
enum SdkInstance {
    /// Real Sdk, using DAPI with gRPC transport
    Dapi {
        /// DAPI client used to communicate with Dash Platform.
        dapi: DapiClient,

        /// Platform version configured for this Sdk
        version: &'static PlatformVersion,
    },
    /// Mock SDK
    Mock {
        /// Mock DAPI client used to communicate with Dash Platform.
        ///
        /// Dapi client is wrapped in a tokio [Mutex](tokio::sync::Mutex) as it's used in async context.
        dapi: Arc<Mutex<MockDapiClient>>,
        /// Mock SDK implementation processing mock expectations and responses.
        mock: std::sync::Mutex<MockDashPlatformSdk>,
    },
}

impl Sdk {
    /// Initialize Dash Platform  SDK in mock mode.
    ///
    /// This is a helper method that uses [`SdkBuilder`] to initialize the SDK in mock mode.
    ///
    /// See also [`SdkBuilder`].
    pub fn new_mock() -> Arc<Self> {
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
    pub(crate) fn parse_proof<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: Mockable,
    {
        let guard = self
            .context_provider
            .lock()
            .expect("context provider lock poisoned");
        let provider = guard
            .as_ref()
            .ok_or(drive_proof_verifier::Error::ContextProviderNotSet)?;

        match self.inner {
            SdkInstance::Dapi { .. } => {
                O::maybe_from_proof(request, response, self.version(), &provider)
            }
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { .. } => self.mock().parse_proof(request, response),
        }
    }

    /// Returns a mutable reference to the `MockDashPlatformSdk` instance.
    ///
    /// Use returned object to configure mock responses with methods like `expect_fetch`.
    ///
    /// # Panics
    ///
    /// Panics if the `self` instance is not a `Mock` variant.
    #[cfg(feature = "mocks")]
    pub fn mock(&self) -> std::sync::MutexGuard<MockDashPlatformSdk> {
        if let Sdk {
            inner: SdkInstance::Mock { ref mock, .. },
            ..
        } = self
        {
            mock.lock().expect("mock lock poisoned")
        } else {
            panic!("not a mock")
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
            SdkInstance::Mock { .. } => self.mock().version(),
        }
    }

    /// Indicate if the sdk should request and verify proofs.
    pub fn prove(&self) -> bool {
        self.proofs
    }

    /// Set the wallet to use.
    ///
    /// Wallet is used to manage keys and addresses and sign transactions.
    /// It can be changed on runtime with set_wallet().
    ///
    /// Note that this will overwrite any previous wallet.
    pub async fn set_wallet<W: Wallet + 'static>(&self, wallet: W) {
        let mut lock = self.wallet.lock().await;
        lock.replace(Box::new(wallet));
    }

    /// Set the [ContextProvider] to use.
    ///
    /// [ContextProvider] is used to access state information, like data contracts and quorum public keys.
    ///
    /// Note that this will overwrite any previous context provider.
    pub fn set_context_provider<C: ContextProvider + 'static>(&self, context_provider: C) {
        let mut guard = self
            .context_provider
            .lock()
            .expect("context provider lock poisoned");

        guard.deref_mut().replace(Box::new(context_provider));
    }

    /// Save quorum public key to disk.
    ///
    /// Files are named: `quorum_pubkey-<int_quorum_type>-<hex_quorum_hash>.json`
    ///
    /// Note that this will overwrite files with the same quorum type and quorum hash.
    ///
    /// Any errors are logged on `warn` level and ignored.
    #[cfg(feature = "mocks")]
    fn dump_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        _core_chain_locked_height: u32,
        public_key: &[u8],
    ) {
        let path = match &self.dump_dir {
            Some(p) => p,
            None => return,
        };

        let encoded = serde_json::to_vec(public_key).expect("encode quorum hash to json");

        let file = path.join(format!(
            "quorum_pubkey-{}-{}.json",
            quorum_type,
            quorum_hash.encode_hex::<String>()
        ));

        if let Err(e) = std::fs::write(file, encoded) {
            tracing::warn!("Unable to write dump file {:?}: {}", path, e);
        }
    }

    /// Create a new [`TransitionContext`] with default configuration.
    ///
    /// This method is used to create a new [`TransitionContext`] that stores various parameters
    /// required to execute a state transition.
    // pub fn create_transition_context(&self) -> TransitionContext {
    //     TransitionContext::new(self)
    // }

    /// Return a wallet lock guard that allows access to the wallet.
    pub async fn wallet(&self) -> tokio::sync::MutexGuard<Option<Box<dyn Wallet>>> {
        self.wallet.lock().await
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

impl ContextProvider for Sdk {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], ContextProviderError> {
        let guard = self
            .context_provider
            .lock()
            .expect("context provider lock poisoned");
        let provider = guard.as_ref().ok_or(ContextProviderError::Config(
            "context provider not configured in sdk, use SdkBuilder::with_context_provider()"
                .to_string(),
        ))?;

        let key: [u8; 48] =
            provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)?;

        #[cfg(feature = "mocks")]
        self.dump_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height, &key);

        Ok(key)
    }

    fn get_data_contract(
        &self,
        data_contract_id: &Identifier,
    ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
        let guard = self
            .context_provider
            .lock()
            .expect("context provider lock poisoned");
        let provider = guard.as_ref().ok_or(ContextProviderError::Config(
            "context provider not configured in sdk, use SdkBuilder::with_context_provider()"
                .to_string(),
        ))?;

        provider.get_data_contract(data_contract_id)
    }
}

#[async_trait::async_trait]
impl Dapi for Sdk {
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

    core_ip: String,
    core_port: u16,
    core_user: String,
    core_password: String,

    /// If true, request and verify proofs of the responses.
    proofs: bool,

    /// Platform version to use in this Sdk
    version: &'static PlatformVersion,

    /// Cache settings
    data_contract_cache_size: NonZeroUsize,
    quorum_public_keys_cache_size: NonZeroUsize,

    /// Wallet used by the SDK.
    wallet: Option<Box<dyn Wallet>>,

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
            core_ip: "".to_string(),
            core_port: 0,
            core_password: "".to_string(),
            core_user: "".to_string(),

            proofs: true,

            data_contract_cache_size: NonZeroUsize::new(DEFAULT_CONTRACT_CACHE_SIZE)
                .expect("data conttact cache size must be positive"),
            quorum_public_keys_cache_size: NonZeroUsize::new(DEFAULT_QUORUM_PUBLIC_KEYS_CACHE_SIZE)
                .expect("quorum public keys cache size must be positive"),

            context_provider: None,
            wallet: None,

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

    /// Configure wallet to use.
    ///
    /// Wallet is used to manage keys and addresses and sign transactions.
    ///
    /// See [Wallet] for more information and [MockWallet] for an example implementation.
    pub fn with_wallet<W: Wallet + 'static>(mut self, wallet: W) -> Self {
        self.wallet = Some(Box::new(wallet));

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

    /// Configure directory where dumps of all requests and responses will be saved.
    /// Useful for debugging.
    ///
    /// This function will create the directory if it does not exist and save dumps of
    /// * all requests and responses - in files named `msg-*.json`
    /// * retrieved quorum public keys - in files named `quorum_pubkey-*.json`
    ///
    /// Data is saved in JSON format.
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
    pub fn build(self) -> Result<Arc<Sdk>, Error> {
        PlatformVersion::set_current(self.version);

        let wallet = self.wallet;

        let  sdk=  match self.addresses {
            // non-mock mode
            Some(addresses) => {
                let dapi = DapiClient::new(addresses, self.settings);
                #[cfg(feature = "mocks")]
                let dapi = dapi.dump_dir(self.dump_dir.clone());

                let sdk= Sdk{
                    inner:SdkInstance::Dapi { dapi,  version:self.version },
                    proofs:self.proofs,
                    context_provider: std::sync:: Mutex::new(self.context_provider),
                    wallet: tokio::sync::Mutex::new(wallet),
                    cancel_token: self.cancel_token,
                    #[cfg(feature = "mocks")]
                    dump_dir: self.dump_dir,
                };
                let sdk = Arc::new(sdk);

                // if context provider is not set correctly (is None), it means we need to fallback to core wallet
                let mut ctx_guard = sdk.context_provider.lock().expect("lock poisoned");
                if  ctx_guard.is_none() { if !self.core_ip.is_empty() {
                    tracing::warn!("ContextProvider not set; mocking with Dash Core. \
                    Please provide your own ContextProvider with SdkBuilder::with_context_provider().");

                    let context_provider = GrpcContextProvider::new(Some(Arc::clone(&sdk)),
                    &self.core_ip, self.core_port, &self.core_user, &self.core_password,
                    self.data_contract_cache_size, self.quorum_public_keys_cache_size)?;
                    ctx_guard.replace(Box::new(context_provider));
                } else{
                    tracing::warn!(
                        "Configure ContextProvider with Sdk::with_context_provider(); otherwise Sdk will fail"
                            );
                }
            };
            drop(ctx_guard);


            sdk
            },
            #[cfg(feature = "mocks")]
            // mock mode
            None =>{ let dapi =Arc::new(tokio::sync::Mutex::new(  MockDapiClient::new()));
                // We create mock context provider that will use the mock DAPI client to retrieve data contracts.
                let  context_provider = self.context_provider.unwrap_or(Box::new(MockContextProvider::new()));

          let sdk= Sdk{
                inner:SdkInstance::Mock {
                    mock: std::sync::Mutex::new( MockDashPlatformSdk::new(self.version, Arc::clone(&dapi), self.proofs)),
                    dapi,
                },
                dump_dir: self.dump_dir,
                proofs:self.proofs,
                context_provider:  std::sync:: Mutex::new( Some(context_provider)),
                wallet: tokio::sync::Mutex::new(wallet),
                cancel_token: self.cancel_token,
            };
            Arc::new(sdk)
        },
            #[cfg(not(feature = "mocks"))]
            None => Err(Error::Config(
                "Mock mode is not available. Please enable `mocks` feature or provide address list.".to_string(),
            )),
        };

        Ok(sdk)
    }
}
