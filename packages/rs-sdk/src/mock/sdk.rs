//! Mocking mechanisms for Dash Platform SDK.
//!
//! See [MockDashPlatformSdk] for more details.
use super::MockResponse;
use crate::{
    platform::{
        types::{evonode::EvoNode, identity::IdentityRequest},
        Fetch, FetchMany, Query,
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
    /// You have to call [MockDashPlatformSdk::set_sdk()] to set sdk, otherwise Mock SDK will panic.
    pub(crate) fn new(dapi: Arc<Mutex<MockDapiClient>>) -> Self {
        Self {
            from_proof_expectations: Default::default(),
            dapi,
            sdk: ArcSwapOption::new(None),
        }
    }

    pub(crate) fn set_sdk(&mut self, sdk: Sdk) {
        self.sdk.store(Some(Arc::new(sdk)));
    }

    /// Returns the current `PlatformVersion` from the outer [`Sdk`]'s
    /// auto-detect-aware atomic. Both request-encode (`sdk.query_settings()`)
    /// and proof-decode (`parse_proof_with_metadata`) read through this
    /// single source, so a mock ratchet from response metadata is visible
    /// to both paths.
    ///
    /// ## Panics
    ///
    /// Panics when sdk is not set during initialization.
    pub(crate) fn version<'v>(&self) -> &'v PlatformVersion {
        if let Some(sdk) = self.sdk.load().as_ref() {
            sdk.version()
        } else {
            panic!("sdk must be set when creating mock ")
        }
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
                "GetDocumentsRequest" => {
                    load_expectation::<proto::GetDocumentsRequest>(&mut dapi, filename)?
                }
                "GetChainedDocumentsRequest" => {
                    load_expectation::<proto::GetChainedDocumentsRequest>(&mut dapi, filename)?
                }
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
                "GetDocumentHistoryRequest" => {
                    load_expectation::<proto::GetDocumentHistoryRequest>(&mut dapi, filename)?
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
                "GetAddressInfoRequest" => {
                    load_expectation::<proto::GetAddressInfoRequest>(&mut dapi, filename)?
                }
                "GetAddressesInfosRequest" => {
                    load_expectation::<proto::GetAddressesInfosRequest>(&mut dapi, filename)?
                }
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
                "GetTokenPreProgrammedDistributionsRequest" => {
                    load_expectation::<proto::GetTokenPreProgrammedDistributionsRequest>(
                        &mut dapi, filename,
                    )?
                }
                "GetAddressesTrunkStateRequest" => {
                    load_expectation::<proto::GetAddressesTrunkStateRequest>(&mut dapi, filename)?
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
    /// - `Q`: Type of the query that will be sent to Platform. Must implement [Query].
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
    pub async fn expect_fetch<O: Fetch + MockResponse, Q: Query<<O as Fetch>::Query>>(
        &mut self,
        query: Q,
        object: Option<O>,
    ) -> Result<&mut Self, Error>
    where
        <<O as Fetch>::Request as TransportRequest>::Response: Default,
    {
        let (rich, wire) =
            self.encode_rich_to_wire::<Q, <O as Fetch>::Query, <O as Fetch>::Request>(query);
        self.expect(&rich, wire, object).await?;

        Ok(self)
    }

    /// Remove previously defined expectation for a [Fetch] request.
    ///
    /// Returns `true` if any expectation was removed.
    pub async fn remove_fetch_expectation<O, Q>(&mut self, query: Q) -> bool
    where
        O: Fetch,
        Q: Query<<O as Fetch>::Query>,
    {
        let (rich, wire) =
            self.encode_rich_to_wire::<Q, <O as Fetch>::Query, <O as Fetch>::Request>(query);
        self.remove(&rich, wire).await
    }

    /// Expect a [FetchMany] request and return provided object.
    ///
    /// This method is used to define mock expectations for [FetchMany] requests.
    ///
    /// ## Generic Parameters
    ///
    /// - `O`: Type of the object that will be returned in response to the query.
    ///   Must implement [FetchMany].
    /// - `Q`: Type of the query that will be sent to Platform. Must implement [Query].
    /// - `R`: Collection type for the results. Must implement [MockResponse].
    ///
    /// ## Arguments
    ///
    /// - `query`: Query that will be sent to Platform.
    /// - `objects`: Collection of objects that will be returned in response to `query`, or None if no objects are expected.
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
        Q: Query<<O as FetchMany<K, R>>::Query>,
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
                <O as FetchMany<K, R>>::Query,
                Request = <O as FetchMany<K, R>>::Query,
                Response = <<O as FetchMany<K, R>>::Request as TransportRequest>::Response,
            > + Sync
            + Send
            + Default,
        <<O as FetchMany<K, R>>::Request as TransportRequest>::Response: Default,
    {
        let (rich, wire) = self
            .encode_rich_to_wire::<Q, <O as FetchMany<K, R>>::Query, <O as FetchMany<K, R>>::Request>(
                query,
            );
        self.expect(&rich, wire, objects).await?;

        Ok(self)
    }

    /// Encode a user-facing `query` first into its rich form (`R`) and
    /// then into its wire form (`W`), both against the SDK's current
    /// `QuerySettings`. Returns `(rich, wire)` for use as proof-mock /
    /// DAPI-mock expectation keys.
    ///
    /// ## Panics
    ///
    /// INTENTIONAL(SEC-001): test-harness fail-fast — encoder errors
    /// for V1-only `DocumentQuery` features against a V0
    /// `PlatformVersion` crash the test setup loudly rather than
    /// silently propagate. Panics also if `set_sdk` was not called.
    fn encode_rich_to_wire<Q, R, W>(&self, query: Q) -> (R, W)
    where
        Q: Query<R>,
        R: Query<W> + Mockable,
        W: TransportRequest,
    {
        let sdk_guard = self.sdk.load();
        let sdk = sdk_guard
            .as_ref()
            .expect("sdk must be set when creating mock");
        let settings = sdk.query_settings();
        let rich: R = query.query(&settings).expect("query must be correct");
        let wire: W = rich.query(&settings).expect("wire encoding must succeed");
        (rich, wire)
    }

    /// Save expectations for a request.
    ///
    /// `rich_request` is the user-facing query (what [`FromProof`] binds to) and seeds
    /// the proof-mock cache key. `wire_request` is the proto that flows over the wire
    /// and seeds the DAPI executor mock. For non-versioned operations both arguments
    /// are the same value; for documents the rich form is [`DocumentQuery`] and the
    /// wire is [`GetDocumentsRequest`].
    async fn expect<R: Mockable + std::fmt::Debug, W: TransportRequest, O: MockResponse>(
        &mut self,
        rich_request: &R,
        wire_request: W,
        returned_object: Option<O>,
    ) -> Result<(), Error>
    where
        W::Response: Default,
    {
        let key = Key::new(rich_request);

        if self.from_proof_expectations.contains_key(&key) {
            return Err(MockError::MockExpectationConflict(format!(
                "proof expectation key {} already defined for {} request: {:?}",
                key,
                std::any::type_name::<R>(),
                rich_request
            ))
            .into());
        }

        self.from_proof_expectations
            .insert(key, returned_object.mock_serialize(self));

        let mut dapi_guard = self.dapi.lock().await;
        dapi_guard.expect(
            &wire_request,
            &Ok(ExecutionResponse {
                inner: Default::default(),
                retries: 0,
                address: "http://127.0.0.1".parse().expect("failed to parse address"),
            }),
        )?;

        Ok(())
    }

    /// Remove expectations for a request.
    async fn remove<R: Mockable, W: TransportRequest>(
        &mut self,
        rich_request: &R,
        wire_request: W,
    ) -> bool {
        let key = Key::new(rich_request);
        let removed_from_proof = self.from_proof_expectations.remove(&key).is_some();

        let mut dapi_guard = self.dapi.lock().await;
        let removed_from_dapi = dapi_guard.remove(&wire_request);

        removed_from_proof || removed_from_dapi
    }

    /// Wrapper around [FromProof] that uses mock expectations, falling back to [FromProof] if no expectation is found.
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
            // Report the latest protocol version so the proof path's ratchet
            // (`maybe_update_protocol_version`) fires as it would against a real
            // network; `default()` reports 0, which the ratchet ignores.
            Some(d) => (
                Option::<O>::mock_deserialize(self, d),
                ResponseMetadata {
                    protocol_version: dpp::version::LATEST_VERSION,
                    ..Default::default()
                },
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
    /// Return context provider implementation defined for upstream Sdk object.
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
