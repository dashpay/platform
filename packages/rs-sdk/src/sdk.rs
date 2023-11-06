//! [Sdk] entrypoint to Dash Platform.

#[cfg(feature = "mocks")]
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(feature = "mocks")]
use crate::mock::MockDashPlatformSdk;
use crate::mock::{MockRequest, MockResponse};
use crate::{core_client::CoreClient, error::Error};
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
#[cfg(feature = "mocks")]
use drive_proof_verifier::MockQuorumInfoProvider;
use drive_proof_verifier::{FromProof, QuorumInfoProvider};
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
#[cfg(feature = "mocks")]
use tokio::sync::Mutex;

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
/// ## Examples
///
/// See tests/ for examples of using the SDK.
pub struct Sdk {
    inner: SdkInstance,
    /// Use proofs when retrieving data from the platform.
    ///
    /// This is set to `true` by default. `false` is not implemented yet.
    proofs: bool,
    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,
}

/// Internal Sdk instance.
///
/// This is used to store the actual Sdk instance, which can be either a real Sdk or a mock Sdk.
/// We use it to avoid exposing internals defined below to the public.
enum SdkInstance {
    /// Real Sdk, using DAPI with gRPC transport
    Dapi {
        /// DAPI client used to communicate with Dash Platform.
        dapi: DapiClient,
        /// Core client used to retrieve quorum keys from core.
        core: CoreClient,
        /// Platform version configured for this Sdk
        version: &'static PlatformVersion,
    },
    #[cfg(feature = "mocks")]
    /// Mock SDK
    Mock {
        /// Mock DAPI client used to communicate with Dash Platform.
        dapi: Arc<Mutex<MockDapiClient>>,
        /// Mock SDK implementation processing mock expectations and responses.
        mock: MockDashPlatformSdk,
        quorum_provider: MockQuorumInfoProvider,
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
    pub(crate) fn parse_proof<R, O: FromProof<R> + MockResponse>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: MockRequest,
    {
        match self.inner {
            SdkInstance::Dapi { .. } => {
                O::maybe_from_proof(request, response, self.version(), self)
            }
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mock, .. } => mock.parse_proof(request, response),
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
    pub fn mock(&mut self) -> &mut MockDashPlatformSdk {
        if let Sdk {
            inner: SdkInstance::Mock { ref mut mock, .. },
            ..
        } = self
        {
            mock
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
    pub fn version<'a>(&self) -> &'a PlatformVersion {
        match &self.inner {
            SdkInstance::Dapi { version, .. } => version,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { mock, .. } => mock.version(),
        }
    }

    /// Indicate if the sdk should request and verify proofs.
    pub fn prove(&self) -> bool {
        self.proofs
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

        let encoded = serde_json::to_vec_pretty(public_key).expect("encode quorum hash to json");

        let file = path.join(format!(
            "quorum_pubkey-{}-{}.json",
            quorum_type,
            quorum_hash.encode_hex::<String>()
        ));

        if let Err(e) = std::fs::write(file, encoded) {
            tracing::warn!("Unable to write dump file {:?}: {}", path, e);
        }
    }
}

impl QuorumInfoProvider for Sdk {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        let provider: &dyn QuorumInfoProvider = match self.inner {
            SdkInstance::Dapi { ref core, .. } => core,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock {
                ref quorum_provider,
                ..
            } => quorum_provider,
        };

        let key =
            provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)?;

        #[cfg(feature = "mocks")]
        self.dump_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height, &key);

        Ok(key)
    }
}

#[async_trait::async_trait]
impl Dapi for Sdk {
    async fn execute<R: TransportRequest>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>> {
        match self.inner {
            SdkInstance::Dapi { ref mut dapi, .. } => dapi.execute(request, settings).await,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mut dapi, .. } => {
                let mut dapi_guard = dapi.lock().await;
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

    version: &'static PlatformVersion,

    /// directory where dump files will be stored
    #[cfg(feature = "mocks")]
    dump_dir: Option<PathBuf>,
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

    /// Configure connection to Dash Core
    ///
    /// TODO: This is temporary implementation, effective until we integrate SPV into dash-platform-sdk.
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
    pub fn build(self) -> Result<Sdk, Error> {
        PlatformVersion::set_current(self.version);

        match self.addresses {
            Some(addresses) => {
                if self.core_ip.is_empty() || self.core_port == 0 {
                    return Err(Error::Config(
                        "Core must be configured with SdkBuilder::with_core".to_string(),
                    ));
                }
                let dapi = DapiClient::new(addresses, self.settings);
                #[cfg(feature = "mocks")]
                let dapi = dapi.dump_dir(self.dump_dir.clone());

                let core = CoreClient::new(
                    &self.core_ip,
                    self.core_port,
                    &self.core_user,
                    &self.core_password,
                )?;

                Ok(Sdk{
                    inner:SdkInstance::Dapi { dapi, core, version:self.version },
                    proofs:self.proofs,
                    #[cfg(feature = "mocks")]
                    dump_dir: self.dump_dir,
                })
            },
            #[cfg(feature = "mocks")]
            None =>{ let dapi =Arc::new(Mutex::new(  MockDapiClient::new()));
                Ok(Sdk{
                    inner:SdkInstance::Mock {
                        mock: MockDashPlatformSdk::new(self.version, Arc::clone(&dapi), self.proofs),
                        dapi,
                        quorum_provider: MockQuorumInfoProvider::new(),
                    },
                    dump_dir: self.dump_dir,
                    proofs:self.proofs,
            })},
            #[cfg(not(feature = "mocks"))]
            None => Err(Error::Config(
                "Mock mode is not available. Please enable `mocks` feature or provide address list.".to_string(),
            )),
        }
    }
}
