//! Mocking mechanisms for Dash Platform SDK.
//!
//! See [MockDashPlatformSdk] for more details.
use super::MockResponse;
use crate::{
    platform::{
        types::{evonode::EvoNode, identity::IdentityRequest},
        DocumentQuery, Fetch, FetchMany, Query,
    },
    sync::block_on,
    Error, Sdk,
};
use arc_swap::ArcSwapOption;
use dapi_grpc::platform::v0::{Proof, ResponseMetadata};
use dapi_grpc::{
    mock::Mockable,
    platform::v0::{self as proto},
};
use dash_context_provider::{ContextProvider, ContextProviderError};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_proof_verifier::FromProof;
use rs_dapi_client::mock::MockError;
use rs_dapi_client::{
    mock::{Key, MockDapiClient},
    transport::TransportRequest,
    DapiClient, DumpData, ExecutionResponse,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, OwnedMutexGuard};

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
    sdk: ArcSwapOption<Sdk>,
}

impl MockDashPlatformSdk {
    /// Returns true when requests should use proofs.
    ///
    /// ## Panics
    ///
    /// Panics when sdk is not set during initialization.
    pub fn prove(&self) -> bool {
        if let Some(sdk) = self.sdk.load().as_ref() {
            sdk.prove()
        } else {
            panic!("sdk must be set when creating mock ")
        }
    }

    /// Create new mock SDK.
    ///
    /// ## Note
    ///
    /// You have to call [MockDashPlatformSdk::with_sdk()] to set sdk, otherwise Mock SDK will panic.
    pub(crate) fn new(version: &'static PlatformVersion, dapi: Arc<Mutex<MockDapiClient>>) -> Self {
        Self {
            from_proof_expectations: Default::default(),
            platform_version: version,
            dapi,
            sdk: ArcSwapOption::new(None),
        }
    }

    pub(crate) fn set_sdk(&mut self, sdk: Sdk) {
        self.sdk.store(Some(Arc::new(sdk)));
    }

    pub(crate) fn version<'v>(&self) -> &'v PlatformVersion {
        self.platform_version
    }

    /// Load all expectations from files in a directory asynchronously.
    ///
    /// See [MockDashPlatformSdk::load_expectations_sync()] for more details.
    #[deprecated(since = "1.4.0", note = "use load_expectations_sync")]
    pub async fn load_expectations<P: AsRef<std::path::Path> + Send + 'static>(
        &mut self,
        dir: P,
    ) -> Result<&mut Self, Error> {
        self.load_expectations_sync(dir)
    }

    /// Load all expectations from files in a directory.
    ///
    ///
    /// By default, mock expectations are loaded when Sdk is built with [SdkBuilder::build()](crate::SdkBuilder::build()).
    /// This function can be used to load expectations after the Sdk is created, or use alternative location.
    /// Expectation files must be prefixed with [DapiClient::DUMP_FILE_PREFIX] and
    /// have `.json` extension.
    pub fn load_expectations_sync<P: AsRef<std::path::Path>>(
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

        let mut dapi = block_on(self.dapi.clone().lock_owned())?;

        for filename in &files {
            let basename = filename.file_name().unwrap().to_str().unwrap();
            let request_type = basename.split('_').nth(1).unwrap_or_default();

            match request_type {
                "DocumentQuery" => load_expectation::<DocumentQuery>(&mut dapi, filename)?,
                "GetEpochsInfoRequest" => {
                    load_expectation::<proto::GetEpochsInfoRequest>(&mut dapi, filename)?
                }
                "GetDataContractRequest" => {
                    load_expectation::<proto::GetDataContractRequest>(&mut dapi, filename)?
                }
                "GetDataContractsRequest" => {
                    load_expectation::<proto::GetDataContractsRequest>(&mut dapi, filename)?
                }
                "GetDataContractHistoryRequest" => {
                    load_expectation::<proto::GetDataContractHistoryRequest>(&mut dapi, filename)?
                }
                "IdentityRequest" => load_expectation::<IdentityRequest>(&mut dapi, filename)?,
                "GetIdentityRequest" => {
                    load_expectation::<proto::GetIdentityRequest>(&mut dapi, filename)?
                }

                "GetIdentityBalanceRequest" => {
                    load_expectation::<proto::GetIdentityBalanceRequest>(&mut dapi, filename)?
                }
                "GetIdentityContractNonceRequest" => {
                    load_expectation::<proto::GetIdentityContractNonceRequest>(&mut dapi, filename)?
                }
                "GetIdentityBalanceAndRevisionRequest" => load_expectation::<
                    proto::GetIdentityBalanceAndRevisionRequest,
                >(&mut dapi, filename)?,
                "GetIdentityKeysRequest" => {
                    load_expectation::<proto::GetIdentityKeysRequest>(&mut dapi, filename)?
                }
                "GetProtocolVersionUpgradeStateRequest" => load_expectation::<
                    proto::GetProtocolVersionUpgradeStateRequest,
                >(&mut dapi, filename)?,
                "GetProtocolVersionUpgradeVoteStatusRequest" => {
                    load_expectation::<proto::GetProtocolVersionUpgradeVoteStatusRequest>(
                        &mut dapi, filename,
                    )?
                }
                "GetContestedResourcesRequest" => {
                    load_expectation::<proto::GetContestedResourcesRequest>(&mut dapi, filename)?
                }
                "GetContestedResourceVoteStateRequest" => load_expectation::<
                    proto::GetContestedResourceVoteStateRequest,
                >(&mut dapi, filename)?,
                "GetContestedResourceVotersForIdentityRequest" => {
                    load_expectation::<proto::GetContestedResourceVotersForIdentityRequest>(
                        &mut dapi, filename,
                    )?
                }
                "GetContestedResourceIdentityVotesRequest" => {
                    load_expectation::<proto::GetContestedResourceIdentityVotesRequest>(
                        &mut dapi, filename,
                    )?
                }
                "GetVotePollsByEndDateRequest" => {
                    load_expectation::<proto::GetVotePollsByEndDateRequest>(&mut dapi, filename)?
                }
                "GetPrefundedSpecializedBalanceRequest" => load_expectation::<
                    proto::GetPrefundedSpecializedBalanceRequest,
                >(&mut dapi, filename)?,
                "GetPathElementsRequest" => {
                    load_expectation::<proto::GetPathElementsRequest>(&mut dapi, filename)?
                }
                "GetTotalCreditsInPlatformRequest" => load_expectation::<
                    proto::GetTotalCreditsInPlatformRequest,
                >(&mut dapi, filename)?,
                "GetIdentityTokenBalancesRequest" => {
                    load_expectation::<proto::GetIdentityTokenBalancesRequest>(&mut dapi, filename)?
                }
                "GetIdentitiesTokenBalancesRequest" => load_expectation::<
                    proto::GetIdentitiesTokenBalancesRequest,
                >(&mut dapi, filename)?,
                "GetIdentityTokenInfosRequest" => {
                    load_expectation::<proto::GetIdentityTokenInfosRequest>(&mut dapi, filename)?
                }
                "GetIdentitiesTokenInfosRequest" => {
                    load_expectation::<proto::GetIdentitiesTokenInfosRequest>(&mut dapi, filename)?
                }
                "GetTokenStatusesRequest" => {
                    load_expectation::<proto::GetTokenStatusesRequest>(&mut dapi, filename)?
                }
                "GetTokenTotalSupplyRequest" => {
                    load_expectation::<proto::GetTokenTotalSupplyRequest>(&mut dapi, filename)?
                }
                "GetGroupInfoRequest" => {
                    load_expectation::<proto::GetGroupInfoRequest>(&mut dapi, filename)?
                }
                "GetGroupInfosRequest" => {
                    load_expectation::<proto::GetGroupInfosRequest>(&mut dapi, filename)?
                }
                "GetGroupActionsRequest" => {
                    load_expectation::<proto::GetGroupActionsRequest>(&mut dapi, filename)?
                }
                "GetGroupActionSignersRequest" => {
                    load_expectation::<proto::GetGroupActionSignersRequest>(&mut dapi, filename)?
                }
                "EvoNode" => load_expectation::<EvoNode>(&mut dapi, filename)?,
                "GetTokenDirectPurchasePricesRequest" => load_expectation::<
                    proto::GetTokenDirectPurchasePricesRequest,
                >(&mut dapi, filename)?,
                "GetTokenPerpetualDistributionLastClaimRequest" => {
                    load_expectation::<proto::GetTokenPerpetualDistributionLastClaimRequest>(
                        &mut dapi, filename,
                    )?
                }
                _ => {
                    return Err(Error::Config(format!(
                        "unknown request type {} in {}, missing match arm in load_expectations?",
                        request_type,
                        filename.display()
                    )))
                }
            };
        }

        Ok(self)
    }

    /// Expect a [Fetch] request and return provided object.
    ///
    /// This method is used to define mock expectations for [Fetch] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query. Must implement [Fetch] and [MockResponse].
    /// - `Q`: Type of the query that will be sent to Platform. Must implement [Query] and [Mockable].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to Platform.
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
    ///     use dash_sdk::{Sdk, platform::{Identity, Fetch, dpp::identity::accessors::IdentityGettersV0}};
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
        let grpc_request = query.query(self.prove()).expect("query must be correct");
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
    ///   Must implement [FetchMany]. `Vec<O>` must implement [MockResponse].
    /// - `Q`: Type of the query that will be sent to Platform. Must implement [Query] and [Mockable].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to Platform.
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
        O: FetchMany<K, R>,
        Q: Query<<O as FetchMany<K, R>>::Request>,
        R,
    >(
        &mut self,
        query: Q,
        objects: Option<R>,
    ) -> Result<&mut Self, Error>
    where
        R: FromIterator<(K, Option<O>)>
            + MockResponse
            + FromProof<
                <O as FetchMany<K, R>>::Request,
                Request = <O as FetchMany<K, R>>::Request,
                Response = <<O as FetchMany<K, R>>::Request as TransportRequest>::Response,
            > + Sync
            + Send
            + Default,
        <<O as FetchMany<K, R>>::Request as TransportRequest>::Response: Default,
    {
        let grpc_request = query.query(self.prove()).expect("query must be correct");
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
        dapi_guard.expect(
            &grpc_request,
            &Ok(ExecutionResponse {
                inner: Default::default(),
                retries: 0,
                address: "http://127.0.0.1".parse().expect("failed to parse address"),
            }),
        )?;

        Ok(())
    }

    /// Wrapper around [FromProof] that uses mock expectations instead of executing [FromProof] trait.
    pub(crate) fn parse_proof_with_metadata<I, O: FromProof<I>>(
        &self,
        request: O::Request,
        response: O::Response,
    ) -> Result<(Option<O>, ResponseMetadata, Proof), drive_proof_verifier::Error>
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
                Proof::default(),
            ),
            None => {
                let version = self.version();
                let provider = self.context_provider()
                    .ok_or(ContextProviderError::InvalidQuorum(
                        "expectation not found and quorum info provider not initialized with sdk.mock().quorum_info_dir()".to_string()
                    ))?;
                O::maybe_from_proof_with_metadata(
                    request,
                    response,
                    Network::Regtest,
                    version,
                    &provider,
                )?
            }
        };

        Ok(data)
    }
    /// Return context provider implementation defined for upstreeam Sdk object.
    fn context_provider(&self) -> Option<impl ContextProvider> {
        if let Some(sdk) = self.sdk.load_full() {
            sdk.clone().context_provider()
        } else {
            None
        }
    }
}

/// Load expectation from file and save it to `dapi_guard` mock Dapi client.
///
/// This function is used to load expectations from files in a directory.
/// It is implemented without reference to the `MockDashPlatformSdk` object
/// to make it easier to use in async context.
fn load_expectation<T: TransportRequest>(
    dapi_guard: &mut OwnedMutexGuard<MockDapiClient>,
    path: &PathBuf,
) -> Result<(), Error> {
    let data = DumpData::<T>::load(path)
        .map_err(|e| {
            Error::Config(format!(
                "cannot load mock expectations from {}: {}",
                path.display(),
                e
            ))
        })?
        .deserialize();
    dapi_guard.expect(&data.0, &data.1)?;
    Ok(())
}
