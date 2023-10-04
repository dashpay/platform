//! Dash API implementation
#[cfg(feature = "mocks")]
use crate::mock::MockDashPlatformSdk;
use crate::{core::CoreClient, error::Error};
use dpp::version::{PlatformVersion, PlatformVersionCurrentVersion};
use drive_proof_verifier::{FromProof, QuorumInfoProvider};
use rs_dapi_client::{
    transport::{TransportClient, TransportRequest},
    Dapi, DapiClient, DapiClientError, RequestSettings,
};

pub use http::Uri;
pub use rs_dapi_client::AddressList;

/// Dash Platform SDK
///
/// This is the main entry point for interacting with Dash Platform.
/// It can be initialized in two modes:
/// - `Normal`: Connects to a remote Dash Platform node.
/// - `Mock`: Uses a mock implementation of Dash Platform.
///
/// To initialize the SDK, use [`SdkBuilder`](crate::sdk::SdkBuilder).
///
/// ## Examples
///
/// See tests/ for examples of using the SDK.
pub enum Sdk {
    Dapi {
        dapi: DapiClient,
        core: CoreClient,
    },
    #[cfg(feature = "mocks")]
    Mock {
        mock: MockDashPlatformSdk,
    },
}

impl Sdk {
    /// Initialize Dash Platform  SDK in mock mode.
    ///
    /// This is a helper method that uses [`SdkBuilder`](crate::sdk::SdkBuilder) to initialize the SDK in mock mode.
    ///
    /// See also [`SdkBuilder`](crate::sdk::SdkBuilder).
    pub fn new_mock() -> Self {
        SdkBuilder::new_mock()
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
    pub(crate) fn parse_proof<R, O: FromProof<R>>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<Option<O>, drive_proof_verifier::Error>
    where
        O::Request: serde::Serialize,
        O: for<'de> serde::Deserialize<'de>,
    {
        match self {
            Self::Dapi { ref core, .. } => {
                let provider: &dyn QuorumInfoProvider = core;
                O::maybe_from_proof(request, response, provider)
            }
            #[cfg(feature = "mocks")]
            Self::Mock { ref mock, .. } => mock.parse_proof(request, response),
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
        if let Self::Mock { ref mut mock, .. } = self {
            mock
        } else {
            panic!("not a mock")
        }
    }

    pub fn version<'a>() -> &'a PlatformVersion {
        PlatformVersion::get_current()
            .expect("Dash Platform version not initialized properly, this should never happen")
    }
}

impl QuorumInfoProvider for Sdk {
    fn get_quorum_public_key(
        &self,
        quorum_type: u32,
        quorum_hash: [u8; 32],
        core_chain_locked_height: u32,
    ) -> Result<[u8; 48], drive_proof_verifier::Error> {
        let provider: &dyn QuorumInfoProvider = match self {
            Self::Dapi { ref core, .. } => core,
            #[cfg(feature = "mocks")]
            Self::Mock { ref mock, .. } => mock,
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
        match self {
            Self::Dapi { ref mut dapi, .. } => dapi.execute(request, settings).await,
            #[cfg(feature = "mocks")]
            Self::Mock { ref mut mock, .. } => mock.execute(request, settings).await,
        }
    }
}

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

    /// Create a new SdkBuilder that will connect to testnet.
    ///
    /// Use for testing only.
    pub fn new_testnet() -> Self {
        unimplemented!(
            "Testnet address list not implemented yet. Use new() and provide address list."
        )
    }

    /// Create a new SdkBuilder that will connect to mainnet (production network).
    pub fn new_mainnet() -> Self {
        unimplemented!(
            "Mainnet address list not implemented yet. Use new() and provide address list."
        )
    }

    pub fn new_mock() -> Self {
        Self::default()
    }

    pub fn with_settings(mut self, settings: RequestSettings) -> Self {
        self.settings = settings;
        self
    }

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

                Ok(Sdk::Dapi { dapi, core })
            }
            #[cfg(feature = "mocks")]
            None => Ok(Sdk::Mock {
                mock: MockDashPlatformSdk::new(),
            }),
            #[cfg(not(feature = "mocks"))]
            None => Err(Error::Config(
                "Mock mode is not available. Please enable `mocks` feature or provide address list.".to_string(),
            )),
        }
    }
}
