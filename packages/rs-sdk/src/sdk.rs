//! Dash API implementation
use std::sync::Arc;

#[cfg(feature = "mocks")]
use crate::mock::MockDashPlatformSdk;
use crate::{
    core::CoreClient,
    error::Error,
    mock::{MockRequest, MockResponse},
};
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive_proof_verifier::{FromProof, QuorumInfoProvider};
use rs_dapi_client::{
    mock::MockDapiClient,
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClient, DapiClientError, RequestSettings,
};

pub use http::Uri;
pub use rs_dapi_client::AddressList;
use tokio::sync::Mutex;

/// Dash Platform SDK
///
/// This is the main entry point for interacting with Dash Platform.
/// It can be initialized in two modes:
/// - `Normal`: Connects to a remote Dash Platform node.
/// - `Mock`: Uses a mock implementation of Dash Platform.
///
/// Recommended method of initialization is to use [`SdkBuilder`](crate::sdk::SdkBuilder). There are also some helper
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
pub struct Sdk(SdkInstance);

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
    },
}

impl Sdk {
    /// Initialize Dash Platform  SDK in mock mode.
    ///
    /// This is a helper method that uses [`SdkBuilder`] to initialize the SDK in mock mode.
    ///
    /// See also [`SdkBuilder`](crate::sdk::SdkBuilder).
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
        match self.0 {
            SdkInstance::Dapi { ref core, .. } => {
                let provider: &dyn QuorumInfoProvider = core;
                O::maybe_from_proof(request, response, provider)
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
        if let Sdk(SdkInstance::Mock { ref mut mock, .. }) = self {
            mock
        } else {
            panic!("not a mock")
        }
    }

    /// Return [Dash Platform version](PlatformVersion) information used by this SDK.
    ///
    ///
    ///
    /// This is the version configured in [`SdkBuilder`](crate::sdk::SdkBuilder).
    /// Useful whenever you need to provide [PlatformVersion] to other SDK and DPP methods.
    pub fn version<'a>(&self) -> &'a PlatformVersion {
        match &self.0 {
            SdkInstance::Dapi { version, .. } => version,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { mock, .. } => mock.version(),
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
        let provider: &dyn QuorumInfoProvider = match self.0 {
            SdkInstance::Dapi { ref core, .. } => core,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mock, .. } => mock,
        };

        provider.get_quorum_public_key(quorum_type, quorum_hash, core_chain_locked_height)
    }
}

#[async_trait::async_trait]
impl Dapi for Sdk {
    async fn execute<R: TransportRequest>(
        &mut self,
        request: R,
        settings: RequestSettings,
    ) -> Result<R::Response, DapiClientError<<R::Client as TransportClient>::Error>> {
        match self.0 {
            SdkInstance::Dapi { ref mut dapi, .. } => dapi.execute(request, settings).await,
            #[cfg(feature = "mocks")]
            SdkInstance::Mock { ref mut dapi, .. } => {
                let mut dapi_guard = dapi.lock().await;
                dapi_guard.execute(request, settings).await
            }
        }
    }
}

/// Dash Platform SDK Builder, used to configure and [`build()`] the [Sdk].
///
/// [SdkBuilder] implemenents a "builder" design pattern to allow configuration of the Sdk before it is instantiated.
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

    version: &'static PlatformVersion,
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

            version: PlatformVersion::latest(),
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
    /// TODO: This is temporary implementation, effective until we integrate SPV into rs-sdk.
    pub fn with_core(mut self, ip: &str, port: u16, user: &str, password: &str) -> Self {
        self.core_ip = ip.to_string();
        self.core_port = port;
        self.core_user = user.to_string();
        self.core_password = password.to_string();

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
                let core = CoreClient::new(
                    &self.core_ip,
                    self.core_port,
                    &self.core_user,
                    &self.core_password,
                )?;

                Ok(Sdk(SdkInstance::Dapi { dapi, core, version:self.version }))
            }
            #[cfg(feature = "mocks")]
            None =>{ let dapi =Arc::new(Mutex::new(  MockDapiClient::new()));
                Ok(Sdk(SdkInstance::Mock {
                mock: MockDashPlatformSdk::new(self.version, Arc::clone(&dapi)),
                dapi,
            }))},
            #[cfg(not(feature = "mocks"))]
            None => Err(Error::Config(
                "Mock mode is not available. Please enable `mocks` feature or provide address list.".to_string(),
            )),
        }
    }
}
