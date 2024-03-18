//! Mocking mechanisms for Dash Platform SDK.
//!
//! See [MockDashPlatformSdk] for more details.
use dapi_grpc::platform::v0::ResponseMetadata;
use dapi_grpc::{
    mock::Mockable,
    platform::v0::{self as proto},
};
use dpp::version::PlatformVersion;
use drive_proof_verifier::{error::ContextProviderError, FromProof, MockContextProvider};
use rs_dapi_client::mock::MockError;
use rs_dapi_client::{
    mock::{Key, MockDapiClient},
    transport::TransportRequest,
    DapiClient, DumpData,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

use crate::{
    platform::{types::identity::IdentityRequest, DocumentQuery, Fetch, FetchMany, Query},
    Error,
};

use super::MockResponse;

/// Mechanisms to mock Dash Platform SDK.
///
/// This object is returned by [Sdk::mock()](crate::Sdk::mock()) and is used to define mock expectations.
///
/// Use [MockDashPlatformSdk::expect_fetch_many()] to define expectations for [FetchMany] requests
/// and [MockDashPlatformSdk::expect_fetch()] for [Fetch] requests.
///
/// ## Panics
///
/// Can panic on errors.
#[derive(Debug)]
pub struct MockDashPlatformSdk {
    from_proof_expectations: BTreeMap<Key, Vec<u8>>,
    platform_version: &'static PlatformVersion,
    dapi: Arc<Mutex<MockDapiClient>>,
    prove: bool,
    quorum_provider: Option<MockContextProvider>,
}

impl MockDashPlatformSdk {
    pub(crate) fn new(
        version: &'static PlatformVersion,
        dapi: Arc<Mutex<MockDapiClient>>,
        prove: bool,
    ) -> Self {
        Self {
            from_proof_expectations: Default::default(),
            platform_version: version,
            dapi,
            prove,
            quorum_provider: None,
        }
    }

    pub(crate) fn version<'v>(&self) -> &'v PlatformVersion {
        self.platform_version
    }
    /// Define a directory where files containing quorum information, like quorum public keys, are stored.
    ///
    /// This directory will be used to load quorum information from files.
    /// You can use [SdkBuilder::with_dump_dir()](crate::SdkBuilder::with_dump_dir()) to generate these files.
    pub fn quorum_info_dir<P: AsRef<std::path::Path>>(&mut self, dir: P) -> &mut Self {
        let mut provider = MockContextProvider::new();
        provider.quorum_keys_dir(Some(dir.as_ref().to_path_buf()));
        self.quorum_provider = Some(provider);

        self
    }

    /// Load all expectations from files in a directory.
    ///
    /// Expectation files must be prefixed with [DapiClient::DUMP_FILE_PREFIX] and
    /// have `.json` extension.
    pub async fn load_expectations<P: AsRef<std::path::Path>>(
        &mut self,
        dir: P,
    ) -> Result<&mut Self, Error> {
        let prefix = DapiClient::DUMP_FILE_PREFIX;

        let entries = dir.as_ref().read_dir().map_err(|e| {
            Error::Config(format!(
                "cannot load mock expectations from {}: {}",
                dir.as_ref().display(),
                e
            ))
        })?;

        let files: Vec<PathBuf> = entries
            .into_iter()
            .filter_map(|x| x.ok())
            .filter(|f| {
                f.file_type().is_ok_and(|t| t.is_file())
                    && f.file_name().to_string_lossy().starts_with(prefix)
                    && f.file_name().to_string_lossy().ends_with(".json")
            })
            .map(|f| f.path())
            .collect();

        for filename in &files {
            let basename = filename.file_name().unwrap().to_str().unwrap();
            let request_type = basename.split('_').nth(2).unwrap_or_default();

            match request_type {
                "DocumentQuery" => self.load_expectation::<DocumentQuery>(filename).await?,
                "GetEpochsInfoRequest" => {
                    self.load_expectation::<proto::GetEpochsInfoRequest>(filename)
                        .await?
                }
                "GetDataContractRequest" => {
                    self.load_expectation::<proto::GetDataContractRequest>(filename)
                        .await?
                }
                "GetDataContractsRequest" => {
                    self.load_expectation::<proto::GetDataContractsRequest>(filename)
                        .await?
                }
                "IdentityRequest" => self.load_expectation::<IdentityRequest>(filename).await?,
                "GetIdentityRequest" => {
                    self.load_expectation::<proto::GetIdentityRequest>(filename)
                        .await?
                }

                "GetIdentityBalanceRequest" => {
                    self.load_expectation::<proto::GetIdentityBalanceRequest>(filename)
                        .await?
                }
                "GetIdentityContractNonceRequest" => {
                    self.load_expectation::<proto::GetIdentityContractNonceRequest>(filename)
                        .await?
                }
                "GetIdentityBalanceAndRevisionRequest" => {
                    self.load_expectation::<proto::GetIdentityBalanceAndRevisionRequest>(filename)
                        .await?
                }
                "GetIdentityKeysRequest" => {
                    self.load_expectation::<proto::GetIdentityKeysRequest>(filename)
                        .await?
                }
                "GetProtocolVersionUpgradeStateRequest" => {
                    self.load_expectation::<proto::GetProtocolVersionUpgradeStateRequest>(filename)
                        .await?
                }
                "GetProtocolVersionUpgradeVoteStatusRequest" => {
                    self.load_expectation::<proto::GetProtocolVersionUpgradeVoteStatusRequest>(
                        filename,
                    )
                    .await?
                }
                _ => {
                    return Err(Error::Config(format!(
                        "unknown request type {} in {}",
                        request_type,
                        filename.display()
                    )))
                }
            };
        }

        Ok(self)
    }

    async fn load_expectation<T: TransportRequest>(&mut self, path: &PathBuf) -> Result<(), Error> {
        let data = DumpData::<T>::load(path)
            .map_err(|e| {
                Error::Config(format!(
                    "cannot load mock expectations from {}: {}",
                    path.display(),
                    e
                ))
            })?
            .deserialize();

        self.dapi.lock().await.expect(&data.0, &data.1)?;
        Ok(())
    }

    /// Expect a [Fetch] request and return provided object.
    ///
    /// This method is used to define mock expectations for [Fetch] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query. Must implement [Fetch] and [MockResponse].
    /// - `Q`: Type of the query that will be sent to the platform. Must implement [Query] and [Mockable].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to the platform.
    /// - `object`: Object that will be returned in response to `query`, or None if the object is expected to not exist.
    ///
    /// ## Returns
    ///
    /// * Reference to self on success, to allow chaining
    /// * Error when expectation cannot be set or is already defined for this request
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # let r = tokio::runtime::Runtime::new().unwrap();
    /// #
    /// # r.block_on(async {
    ///     use rs_sdk::{Sdk, platform::{Identity, Fetch, dpp::identity::accessors::IdentityGettersV0}};
    ///
    ///     let mut api = Sdk::new_mock();
    ///     // Define expected response
    ///     let expected: Identity = Identity::random_identity(1, None, api.version())
    ///         .expect("create expected identity");
    ///     // Define query that will be sent
    ///     let query = expected.id();
    ///     // Expect that in response to `query`, `expected` will be returned
    ///     api.mock().expect_fetch(query, Some(expected.clone())).await.unwrap();
    ///
    ///     // Fetch the identity
    ///     let retrieved = dpp::prelude::Identity::fetch(&api, query)
    ///         .await
    ///         .unwrap()
    ///         .expect("object should exist");
    ///
    ///     // Check that the identity is the same as expected
    ///     assert_eq!(retrieved, expected);
    /// # });
    /// ```
    pub async fn expect_fetch<O: Fetch + MockResponse, Q: Query<<O as Fetch>::Request>>(
        &mut self,
        query: Q,
        object: Option<O>,
    ) -> Result<&mut Self, Error>
    where
        <<O as Fetch>::Request as TransportRequest>::Response: Default,
    {
        let grpc_request = query.query(self.prove).expect("query must be correct");
        self.expect(grpc_request, object).await?;

        Ok(self)
    }

    /// Expect a [FetchMany] request and return provided object.
    ///
    /// This method is used to define mock expectations for [FetchMany] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query.
    /// Must implement [FetchMany]. `Vec<O>` must implement [MockResponse].
    /// - `Q`: Type of the query that will be sent to the platform. Must implement [Query] and [Mockable].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to the platform.
    /// - `objects`: Vector of objects that will be returned in response to `query`, or None if no objects are expected.
    ///
    /// ## Returns
    ///
    /// * Reference to self on success, to allow chaining
    /// * Error when expectation cannot be set or is already defined for this request
    ///
    /// ## Panics
    ///
    /// Can panic on errors.
    ///
    /// ## Example
    ///
    /// Usage example is similar to
    /// [MockDashPlatformSdk::expect_fetch()], but the expected
    /// object must be a vector of objects.
    pub async fn expect_fetch_many<
        K: Ord,
        O: FetchMany<K>,
        Q: Query<<O as FetchMany<K>>::Request>,
    >(
        &mut self,
        query: Q,
        objects: Option<BTreeMap<K, Option<O>>>,
    ) -> Result<&mut Self, Error>
    where
        BTreeMap<K, Option<O>>: MockResponse,
        <<O as FetchMany<K>>::Request as TransportRequest>::Response: Default,
        BTreeMap<K, Option<O>>: FromProof<
                <O as FetchMany<K>>::Request,
                Request = <O as FetchMany<K>>::Request,
                Response = <<O as FetchMany<K>>::Request as TransportRequest>::Response,
            > + Sync,
    {
        let grpc_request = query.query(self.prove).expect("query must be correct");
        self.expect(grpc_request, objects).await?;

        Ok(self)
    }

    /// Save expectations for a request.
    async fn expect<I: TransportRequest, O: MockResponse>(
        &mut self,
        grpc_request: I,
        returned_object: Option<O>,
    ) -> Result<(), Error>
    where
        I::Response: Default,
    {
        let key = Key::new(&grpc_request);

        // detect duplicates
        if self.from_proof_expectations.contains_key(&key) {
            return Err(MockError::MockExpectationConflict(format!(
                "proof expectation key {} already defined for {} request: {:?}",
                key,
                std::any::type_name::<I>(),
                grpc_request
            ))
            .into());
        }

        // This expectation will work for from_proof
        self.from_proof_expectations
            .insert(key, returned_object.mock_serialize(self));

        // This expectation will work for execute
        let mut dapi_guard = self.dapi.lock().await;
        // We don't really care about the response, as it will be mocked by from_proof, so we provide default()
        dapi_guard.expect(&grpc_request, &Default::default())?;

        Ok(())
    }

    /// Wrapper around [FromProof] that uses mock expectations instead of executing [FromProof] trait.
    pub(crate) fn parse_proof_with_metadata<I, O: FromProof<I>>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<(Option<O>, ResponseMetadata), drive_proof_verifier::Error>
    where
        O::Request: Mockable,
        Option<O>: MockResponse,
        // O: FromProof<<O as FromProof<I>>::Request>,
    {
        let key = Key::new(&request);

        let data = match self.from_proof_expectations.get(&key) {
            Some(d) => (
                Option::<O>::mock_deserialize(self, d),
                ResponseMetadata::default(),
            ),
            None => {
                let version = self.version();
                let provider = self.quorum_provider.as_ref()
                    .ok_or(ContextProviderError::InvalidQuorum(
                        "expectation not found and quorum info provider not initialized with sdk.mock().quorum_info_dir()".to_string()
                    ))?;
                O::maybe_from_proof_with_metadata(request, response, version, provider)?
            }
        };

        Ok(data)
    }
}
