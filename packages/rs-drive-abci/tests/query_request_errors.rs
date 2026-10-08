//! Invalid caller queries must keep healthy nodes available for other requests.
//! The generated tonic server and client run over an in-memory duplex stream.

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_addresses_infos_request::GetAddressesInfosRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::GetDataContractsByRangeRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::GetDataContractsLatestVersionsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
use dapi_grpc::platform::v0::get_group_actions_request::GetGroupActionsRequestV0;
use dapi_grpc::platform::v0::get_group_infos_request::GetGroupInfosRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_response::get_identities_balances_response_v0;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_token_balances_request::GetIdentitiesTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_token_infos_request::GetIdentitiesTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_response::get_identity_keys_response_v0;
use dapi_grpc::platform::v0::get_identity_token_balances_request::GetIdentityTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_token_infos_request::GetIdentityTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_path_elements_request::GetPathElementsRequestV0;
use dapi_grpc::platform::v0::get_path_elements_response::get_path_elements_response_v0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
use dapi_grpc::platform::v0::get_shielded_encrypted_notes_request::GetShieldedEncryptedNotesRequestV0;
use dapi_grpc::platform::v0::get_token_direct_purchase_prices_request::GetTokenDirectPurchasePricesRequestV0;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::GetTokenPreProgrammedDistributionsRequestV0;
use dapi_grpc::platform::v0::get_token_statuses_request::GetTokenStatusesRequestV0;
use dapi_grpc::platform::v0::get_vote_polls_by_end_date_request::GetVotePollsByEndDateRequestV0;
use dapi_grpc::platform::v0::key_request_type::Request as KeyRequestKind;
use dapi_grpc::platform::v0::{self as wire, platform_server::PlatformServer};
use dapi_grpc::platform::v0::{
    get_data_contracts_request, get_epochs_info_request, get_identities_balances_response,
    get_identity_keys_response, get_path_elements_response,
};
use dapi_grpc::tonic::body::Body;
use dapi_grpc::tonic::codegen::http::Request as HttpRequest;
use dapi_grpc::tonic::service::Routes;
use dapi_grpc::tonic::transport::{Endpoint, Server};
use dapi_grpc::tonic::{Code, Request, Status};
use dpp::block::block_info::BlockInfo;
use dpp::dashcore::Network;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::v0::DataContractConfigSettersV0;
use dpp::identifier::Identifier;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::Drive;
use drive_abci::config::PlatformConfig;
use drive_abci::platform_types::platform::Platform;
use drive_abci::query::QueryService;
use drive_abci::rpc::core::DefaultCoreRPC;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use hyper_util::rt::TokioIo;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, PlatformGrpcClient, TransportClient, TransportError,
    TransportRequest,
};
use rs_dapi_client::{
    AddressList, ConnectionPool, DapiClient, DapiClientError, DapiRequestExecutor, RequestSettings,
    Uri,
};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;
use tokio_stream::StreamExt;
use tower::ServiceExt;

#[derive(Debug)]
struct QueryFixture {
    platform: Arc<Platform<DefaultCoreRPC>>,
    _directory: TempDir,
}

impl QueryFixture {
    fn new() -> Self {
        let mut config = PlatformConfig::default_for_network(Network::Testnet);
        config.core.consensus_rpc.host = "http://127.0.0.1".to_owned();
        config.core.consensus_rpc.port = 1;
        let temporary = TestPlatformBuilder::new()
            .with_config(config)
            .build_with_default_rpc();
        let mut state = temporary.platform.state.load_full().as_ref().clone();
        state.current_protocol_version_in_consensus = PlatformVersion::latest().protocol_version;
        temporary.platform.state.store(Arc::new(state));
        temporary
            .platform
            .drive
            .create_initial_state_structure(None, PlatformVersion::latest())
            .expect("initialize deterministic query state");
        Self {
            platform: Arc::new(temporary.platform),
            _directory: temporary.tempdir,
        }
    }

    fn client(&self) -> (PlatformGrpcClient, tokio::task::JoinHandle<()>) {
        self.client_at("http://queries.in-memory".parse().expect("query URI"))
    }

    fn store_contract(&self) -> (Identifier, String) {
        let version = PlatformVersion::latest();
        let mut contract =
            get_data_contract_fixture(Some(Identifier::new([7; 32])), 0, version.protocol_version)
                .data_contract_owned();
        contract.config_mut().set_keeps_history(true);
        contract.config_mut().set_readonly(false);
        self.platform
            .drive
            .apply_contract(
                &contract,
                BlockInfo {
                    time_ms: 1000,
                    ..Default::default()
                },
                true,
                None,
                None,
                version,
            )
            .expect("store a deterministic contract with history");
        (
            contract.id(),
            contract
                .document_types()
                .keys()
                .next()
                .expect("fixture document type")
                .clone(),
        )
    }

    fn client_at(&self, uri: Uri) -> (PlatformGrpcClient, tokio::task::JoinHandle<()>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let service = QueryService::new(Arc::clone(&self.platform));
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            Server::builder()
                .add_service(PlatformServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .expect("serve in-memory query service");
        });
        let mut io = Some(client_io);
        let channel = Endpoint::from_shared(uri.to_string())
            .expect("served node URI")
            .connect_with_connector_lazy(tower::service_fn(move |_: Uri| {
                let io = io.take();
                async move {
                    io.map(TokioIo::new)
                        .ok_or_else(|| std::io::Error::other("single-use query channel"))
                }
            }));
        (PlatformGrpcClient::new(channel), server)
    }

    fn client_with_fault(
        &self,
        uri: Uri,
        code: Code,
    ) -> (PlatformGrpcClient, tokio::task::JoinHandle<()>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let service = QueryService::new(Arc::clone(&self.platform));
        let server = tokio::spawn(async move {
            let incoming = tokio_stream::iter([Ok::<_, std::io::Error>(server_io)])
                .chain(tokio_stream::pending());
            let layer = tower::layer::layer_fn(move |service: Routes| {
                tower::service_fn(move |request: HttpRequest<Body>| {
                    let service = service.clone();
                    async move {
                        if request.uri().path().ends_with("/getDataContracts") {
                            Ok(Status::new(code, "node query failure").into_http())
                        } else {
                            service.oneshot(request).await
                        }
                    }
                })
            });
            Server::builder()
                .layer(layer)
                .add_service(PlatformServer::new(service))
                .serve_with_incoming(incoming)
                .await
                .expect("serve faulted in-memory node");
        });
        let mut io = Some(client_io);
        let channel = Endpoint::from_shared(uri.to_string())
            .expect("served node URI")
            .connect_with_connector_lazy(tower::service_fn(move |_: Uri| {
                let io = io.take();
                async move {
                    io.map(TokioIo::new)
                        .ok_or_else(|| std::io::Error::other("single-use query channel"))
                }
            }));
        (PlatformGrpcClient::new(channel), server)
    }
}

fn record<R>(
    name: &str,
    prove: bool,
    result: Result<R, Status>,
    expected: Code,
    failures: &mut Vec<String>,
) {
    let (code, message) = match result {
        Ok(_) => (Code::Ok, String::new()),
        Err(status) => (status.code(), status.message().to_owned()),
    };
    println!("{name} prove={prove}: {code:?} {message}");
    if code != expected {
        failures.push(format!(
            "{name} prove={prove}: expected {expected:?}, got {code:?}: {message}"
        ));
    }
}

fn record_cap<R>(
    name: &str,
    prove: bool,
    result: Result<R, Status>,
    expected: Code,
    failures: &mut Vec<String>,
) {
    if expected == Code::InvalidArgument {
        if let Err(status) = &result {
            let message = status.message().to_lowercase();
            if !message.contains("limit") && !message.contains("count") {
                failures.push(format!(
                    "{name} refused for something other than its cap: {message}"
                ));
            }
        }
    }
    record(name, prove, result, expected, failures);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_classify_empty_proof_requests_as_caller_errors() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    for prove in [false, true] {
        let expected = if prove {
            Code::InvalidArgument
        } else {
            Code::Ok
        };
        macro_rules! batch {
            ($method:ident, $request:expr) => {
                record(
                    stringify!($method),
                    prove,
                    client.$method(Request::new(($request).into())).await,
                    expected,
                    &mut failures,
                );
            };
        }
        batch!(
            get_data_contracts,
            GetDataContractsRequestV0 { ids: vec![], prove }
        );
        let balance_response = client
            .get_identities_balances(wire::GetIdentitiesBalancesRequest::from(
                GetIdentitiesBalancesRequestV0 { ids: vec![], prove },
            ))
            .await;
        if prove {
            if let Ok(response) = &balance_response {
                let Some(get_identities_balances_response::Version::V0(v0)) =
                    &response.get_ref().version
                else {
                    panic!("missing response version")
                };
                if let Some(get_identities_balances_response_v0::Result::Proof(proof)) = &v0.result
                {
                    let verified = Drive::verify_identity_balances_for_identity_ids::<
                        Vec<([u8; 32], Option<u64>)>,
                        [u8; 32],
                    >(
                        &proof.grovedb_proof, false, &[], PlatformVersion::latest()
                    );
                    println!("empty balances actual Drive verifier: {verified:?}");
                }
            }
        }
        record(
            "get_identities_balances",
            prove,
            balance_response,
            expected,
            &mut failures,
        );
        batch!(
            get_identities_contract_keys,
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![],
                contract_id: vec![1; 32],
                document_type_name: None,
                purposes: vec![0],
                prove,
            }
        );
        batch!(
            get_identity_token_balances,
            GetIdentityTokenBalancesRequestV0 {
                identity_id: vec![1; 32],
                token_ids: vec![],
                prove,
            }
        );
        batch!(
            get_identity_token_infos,
            GetIdentityTokenInfosRequestV0 {
                identity_id: vec![1; 32],
                token_ids: vec![],
                prove,
            }
        );
        batch!(
            get_identities_token_balances,
            GetIdentitiesTokenBalancesRequestV0 {
                token_id: vec![1; 32],
                identity_ids: vec![],
                prove,
            }
        );
        batch!(
            get_identities_token_infos,
            GetIdentitiesTokenInfosRequestV0 {
                token_id: vec![1; 32],
                identity_ids: vec![],
                prove,
            }
        );
        batch!(
            get_token_statuses,
            GetTokenStatusesRequestV0 {
                token_ids: vec![],
                prove
            }
        );
        batch!(
            get_evonodes_proposed_epoch_blocks_by_ids,
            GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                epoch: Some(0),
                ids: vec![],
                prove,
            }
        );
        batch!(
            get_addresses_infos,
            GetAddressesInfosRequestV0 {
                addresses: vec![],
                prove
            }
        );
        let path_response = client
            .get_path_elements(wire::GetPathElementsRequest::from(
                GetPathElementsRequestV0 {
                    path: vec![],
                    keys: vec![],
                    prove,
                },
            ))
            .await;
        if prove {
            if let Ok(response) = &path_response {
                let Some(get_path_elements_response::Version::V0(v0)) = &response.get_ref().version
                else {
                    panic!("missing response version")
                };
                if let Some(get_path_elements_response_v0::Result::Proof(proof)) = &v0.result {
                    let (root, elements) = Drive::verify_elements(
                        &proof.grovedb_proof,
                        vec![],
                        vec![],
                        PlatformVersion::latest(),
                    )
                    .expect("an empty root-path proof remains usable");
                    assert!(elements.is_empty());
                    assert_eq!(
                        root,
                        fixture
                            .platform
                            .drive
                            .grove
                            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                            .unwrap()
                            .expect("state root")
                    );
                }
            }
        }
        record(
            "get_path_elements",
            prove,
            path_response,
            Code::Ok,
            &mut failures,
        );
    }
    server.abort();
    assert!(
        failures.is_empty(),
        "incorrect empty-query statuses:\n{}",
        failures.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_classify_invalid_zero_caps_as_caller_errors() {
    let fixture = QueryFixture::new();
    let (contract_id, _) = fixture.store_contract();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    for prove in [false, true] {
        macro_rules! zero {
            ($method:ident, $request:expr, $expected:expr) => {
                record_cap(
                    stringify!($method),
                    prove,
                    client.$method(Request::new(($request).into())).await,
                    $expected,
                    &mut failures,
                );
            };
            ($method:ident, $request:expr) => {
                zero!($method, $request, Code::InvalidArgument);
            };
        }
        zero!(
            get_data_contract_history,
            GetDataContractHistoryRequestV0 {
                id: contract_id.to_vec(),
                limit: Some(0),
                offset: None,
                start_at_ms: 0,
                prove,
            }
        );
        zero!(
            get_identity_keys,
            GetIdentityKeysRequestV0 {
                identity_id: vec![1; 32],
                request_type: Some(wire::KeyRequestType {
                    request: Some(KeyRequestKind::AllKeys(wire::AllKeys {})),
                }),
                limit: Some(0),
                offset: None,
                prove,
            },
            if prove {
                Code::InvalidArgument
            } else {
                Code::Ok
            }
        );
        zero!(
            get_epochs_info,
            wire::GetEpochsInfoRequest {
                version: Some(get_epochs_info_request::Version::V0(
                    GetEpochsInfoRequestV0 {
                        start_epoch: Some(0),
                        count: 0,
                        ascending: true,
                        prove,
                    }
                )),
            },
            if prove {
                Code::InvalidArgument
            } else {
                Code::Ok
            }
        );
        zero!(
            get_protocol_version_upgrade_vote_status,
            GetProtocolVersionUpgradeVoteStatusRequestV0 {
                start_pro_tx_hash: vec![],
                count: 0,
                prove,
            }
        );
        zero!(
            get_evonodes_proposed_epoch_blocks_by_range,
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: Some(0),
                limit: Some(0),
                start: None,
                prove,
            }
        );
        zero!(
            get_group_infos,
            GetGroupInfosRequestV0 {
                contract_id: vec![1; 32],
                start_at_group_contract_position: None,
                count: Some(0),
                prove,
            }
        );
        zero!(
            get_group_actions,
            GetGroupActionsRequestV0 {
                contract_id: vec![1; 32],
                group_contract_position: 0,
                status: 0,
                start_at_action_id: None,
                count: Some(0),
                prove,
            }
        );
        zero!(
            get_token_pre_programmed_distributions,
            GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: vec![1; 32],
                start_at_info: None,
                limit: Some(0),
                prove,
            }
        );
        zero!(
            get_contested_resource_identity_votes,
            GetContestedResourceIdentityVotesRequestV0 {
                identity_id: vec![1; 32],
                limit: Some(0),
                offset: None,
                order_ascending: true,
                start_at_vote_poll_id_info: None,
                prove,
            }
        );
    }
    server.abort();
    assert!(
        failures.is_empty(),
        "incorrect zero-cap statuses:\n{}",
        failures.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_retain_existing_request_error_and_valid_batch_contracts() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    for prove in [false, true] {
        record(
            "get_data_contracts_latest_versions",
            prove,
            client
                .get_data_contracts_latest_versions(
                    wire::GetDataContractsLatestVersionsRequest::from(
                        GetDataContractsLatestVersionsRequestV0 {
                            ids: vec![],
                            include_contracts: false,
                            prove,
                        },
                    ),
                )
                .await,
            Code::InvalidArgument,
            &mut failures,
        );
        record(
            "get_token_direct_purchase_prices",
            prove,
            client
                .get_token_direct_purchase_prices(wire::GetTokenDirectPurchasePricesRequest::from(
                    GetTokenDirectPurchasePricesRequestV0 {
                        token_ids: vec![],
                        prove,
                    },
                ))
                .await,
            Code::InvalidArgument,
            &mut failures,
        );
        record(
            "get_data_contracts_by_range",
            prove,
            client
                .get_data_contracts_by_range(wire::GetDataContractsByRangeRequest::from(
                    GetDataContractsByRangeRequestV0 {
                        limit: Some(0),
                        start: None,
                        ids_only: true,
                        prove,
                    },
                ))
                .await,
            Code::InvalidArgument,
            &mut failures,
        );
        record(
            "get_vote_polls_by_end_date",
            prove,
            client
                .get_vote_polls_by_end_date(wire::GetVotePollsByEndDateRequest::from(
                    GetVotePollsByEndDateRequestV0 {
                        start_time_info: None,
                        end_time_info: None,
                        limit: Some(0),
                        offset: None,
                        ascending: true,
                        prove,
                    },
                ))
                .await,
            Code::InvalidArgument,
            &mut failures,
        );
        record(
            "get_data_contracts nonempty",
            prove,
            client
                .get_data_contracts(wire::GetDataContractsRequest::from(
                    GetDataContractsRequestV0 {
                        ids: vec![vec![1; 32]],
                        prove,
                    },
                ))
                .await,
            Code::Ok,
            &mut failures,
        );
    }
    server.abort();
    assert!(
        failures.is_empty(),
        "changed existing query contracts:\n{}",
        failures.join("\n")
    );
}

/// Selected node URI; the request serves this node through a separate duplex connection.
struct QueryNodeClient {
    uri: Uri,
}

impl TransportClient for QueryNodeClient {
    fn with_uri(uri: Uri, _pool: &ConnectionPool) -> Result<Self, TransportError> {
        Ok(Self { uri })
    }

    fn with_uri_and_settings(
        uri: Uri,
        _settings: &AppliedRequestSettings,
        pool: &ConnectionPool,
    ) -> Result<Self, TransportError> {
        Self::with_uri(uri, pool)
    }
}

#[derive(Clone, Debug)]
struct ContractsQuery {
    fixture: Arc<QueryFixture>,
    request: wire::GetDataContractsRequest,
    attempts: Arc<Mutex<Vec<Uri>>>,
    first_node_fault: Option<Code>,
    ban_successful_attempt: Option<AddressList>,
}

impl Mockable for ContractsQuery {}

impl TransportRequest for ContractsQuery {
    type Client = QueryNodeClient;
    type Response = wire::GetDataContractsResponse;
    const SETTINGS_OVERRIDES: RequestSettings = RequestSettings::default();

    fn method_name(&self) -> &'static str {
        "get_data_contracts"
    }

    fn execute_transport<'c>(
        self,
        client: &'c mut Self::Client,
        _settings: &AppliedRequestSettings,
    ) -> BoxFuture<'c, Result<Self::Response, TransportError>> {
        let first_attempt = {
            let mut attempts = self.attempts.lock().expect("attempt log");
            attempts.push(client.uri.clone());
            attempts.len() == 1
        };
        let (mut grpc, server) = if let Some(code) = self.first_node_fault.filter(|_| first_attempt)
        {
            let Some(get_data_contracts_request::Version::V0(v0)) = &self.request.version else {
                panic!("valid control request version")
            };
            assert!(
                !v0.ids.is_empty(),
                "fault injection is only for valid nonempty queries"
            );
            self.fixture.client_with_fault(client.uri.clone(), code)
        } else {
            self.fixture.client_at(client.uri.clone())
        };
        let responding_address = client
            .uri
            .clone()
            .try_into()
            .expect("responding node address");
        Box::pin(async move {
            let result = grpc
                .get_data_contracts(self.request)
                .await
                .map(|response| response.into_inner())
                .map_err(TransportError::Grpc);
            server.abort();
            if result.is_ok() {
                if let Some(addresses) = self.ban_successful_attempt {
                    assert!(addresses.ban(&responding_address));
                    assert!(addresses.is_banned(&responding_address));
                }
            }
            result
        })
    }
}

fn query_nodes(count: u16) -> AddressList {
    let mut addresses = AddressList::with_settings(Duration::from_secs(3600));
    for port in 20000..20000 + count {
        addresses.add(
            format!("http://127.0.0.1:{port}")
                .parse()
                .expect("test node"),
        );
    }
    addresses
}

fn grpc_code(error: &DapiClientError) -> Option<Code> {
    match error {
        DapiClientError::Transport(TransportError::Grpc(status)) => Some(status.code()),
        DapiClientError::NoAvailableAddressesToRetry(error) => match error.as_ref() {
            TransportError::Grpc(status) => Some(status.code()),
        },
        _ => None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_keep_all_thirteen_nodes_available_after_three_empty_contract_proof_requests() {
    let query = ContractsQuery {
        fixture: Arc::new(QueryFixture::new()),
        request: GetDataContractsRequestV0 {
            ids: vec![],
            prove: true,
        }
        .into(),
        attempts: Default::default(),
        first_node_fault: None,
        ban_successful_attempt: None,
    };
    let client = DapiClient::new(
        query_nodes(13),
        RequestSettings {
            retries: Some(5),
            ..RequestSettings::default()
        },
    );
    let mut failures = Vec::new();
    tokio::time::timeout(Duration::from_secs(5), async {
        for call in 1..=3 {
            let error = client.execute(query.clone(), RequestSettings::default()).await.expect_err("invalid proof query");
            let bans = client.address_list().ban_info();
            let banned = bans.iter().filter(|ban| ban.banned).count();
            let attempts = query.attempts.lock().expect("attempt log").len();
            println!("empty contracts call={call}: code={:?} retries={} attempts={attempts} banned={banned}/13 address={:?}", grpc_code(&error.inner), error.retries, error.address);
            if grpc_code(&error.inner) != Some(Code::InvalidArgument) || error.retries != 0 || error.address.is_none()
                || attempts != call || bans.iter().any(|ban| ban.banned || ban.ban_count != 0) {
                failures.push(format!("call={call}: {error:?}; attempts={attempts}; banned={banned}/13"));
            }
        }
    }).await.expect("bans cannot expire during the bounded three-call scenario");
    assert!(
        failures.is_empty(),
        "caller errors exhausted healthy nodes:\n{}",
        failures.join("\n")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_ban_and_fail_over_on_actual_node_errors_for_valid_queries() {
    let fixture = Arc::new(QueryFixture::new());
    for code in [Code::Internal, Code::Unavailable] {
        let addresses = query_nodes(13);
        let query = ContractsQuery {
            fixture: Arc::clone(&fixture),
            request: GetDataContractsRequestV0 {
                ids: vec![vec![1; 32]],
                prove: false,
            }
            .into(),
            attempts: Default::default(),
            first_node_fault: Some(code),
            ban_successful_attempt: Some(addresses.clone()),
        };
        let client = DapiClient::new(
            addresses,
            RequestSettings {
                retries: Some(5),
                ..RequestSettings::default()
            },
        );
        let response = tokio::time::timeout(
            Duration::from_secs(5),
            client.execute(query.clone(), RequestSettings::default()),
        )
        .await
        .expect("bounded failover")
        .expect("real query service succeeds on a healthy node");
        let attempts = query.attempts.lock().expect("attempt log");
        assert_eq!(attempts.len(), 2, "{code:?} must fail over once");
        assert_ne!(attempts[0], attempts[1]);
        let failed_node = attempts[0].clone().try_into().expect("failed node address");
        assert!(client.address_list().is_banned(&failed_node));
        assert_eq!(response.retries, 1);
        assert_eq!(response.address.uri(), &attempts[1]);
        assert!(
            !client.address_list().is_banned(&response.address),
            "a successful in-flight attempt must unban its node"
        );
        println!("valid contract query with {code:?}: two attempts, faulty source banned, healthy source unbanned");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_preserve_zero_defaults_optional_caps_and_key_selection() {
    let fixture = QueryFixture::new();
    let (contract_id, document_type) = fixture.store_contract();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    for prove in [false, true] {
        let documents = |limit| {
            wire::GetDocumentsRequest::from(GetDocumentsRequestV0 {
                data_contract_id: contract_id.to_vec(),
                document_type: document_type.clone(),
                r#where: vec![],
                order_by: vec![],
                limit,
                start: None,
                prove,
            })
        };
        let zero = client
            .get_documents(documents(0))
            .await
            .expect("legacy document zero means default")
            .into_inner();
        let explicit = client
            .get_documents(documents(100))
            .await
            .expect("default document cap succeeds")
            .into_inner();
        assert_eq!(
            zero, explicit,
            "documents V0 zero preserves the existing default"
        );
        let notes = |count| {
            wire::GetShieldedEncryptedNotesRequest::from(GetShieldedEncryptedNotesRequestV0 {
                start_index: 0,
                count,
                prove,
                token_id: None,
            })
        };
        let zero = client
            .get_shielded_encrypted_notes(notes(0))
            .await
            .expect("shielded zero means default")
            .into_inner();
        let explicit = client
            .get_shielded_encrypted_notes(notes(u32::MAX))
            .await
            .expect("shielded cap clamps to default")
            .into_inner();
        assert_eq!(
            zero, explicit,
            "shielded count zero preserves the existing default"
        );
        for limit in [None, Some(1)] {
            client
                .get_data_contract_history(wire::GetDataContractHistoryRequest::from(
                    GetDataContractHistoryRequestV0 {
                        id: contract_id.to_vec(),
                        limit,
                        offset: None,
                        start_at_ms: 0,
                        prove,
                    },
                ))
                .await
                .expect("positive and absent historical contract caps remain valid");
            client
                .get_identity_keys(wire::GetIdentityKeysRequest::from(
                    GetIdentityKeysRequestV0 {
                        identity_id: vec![1; 32],
                        request_type: Some(wire::KeyRequestType {
                            request: Some(KeyRequestKind::AllKeys(wire::AllKeys {})),
                        }),
                        limit,
                        offset: None,
                        prove,
                    },
                ))
                .await
                .expect("positive and absent key caps remain valid");
        }
        record(
            "epochs count=1",
            prove,
            client
                .get_epochs_info(wire::GetEpochsInfoRequest {
                    version: Some(get_epochs_info_request::Version::V0(
                        GetEpochsInfoRequestV0 {
                            start_epoch: Some(0),
                            count: 1,
                            ascending: true,
                            prove,
                        },
                    )),
                })
                .await,
            Code::Ok,
            &mut failures,
        );
        record(
            "root path nonempty keys",
            prove,
            client
                .get_path_elements(wire::GetPathElementsRequest::from(
                    GetPathElementsRequestV0 {
                        path: vec![],
                        keys: vec![vec![96]],
                        prove,
                    },
                ))
                .await,
            Code::Ok,
            &mut failures,
        );
        let mut selections = vec![(vec![], None), (vec![], Some(0)), (vec![0, 0], Some(1))];
        if prove {
            selections.extend([(vec![0], Some(0)), (vec![0, 1], Some(1))]);
        }
        for (key_ids, limit) in selections {
            let result = client
                .get_identity_keys(wire::GetIdentityKeysRequest::from(
                    GetIdentityKeysRequestV0 {
                        identity_id: vec![1; 32],
                        request_type: Some(wire::KeyRequestType {
                            request: Some(KeyRequestKind::SpecificKeys(wire::SpecificKeys {
                                key_ids: key_ids.clone(),
                            })),
                        }),
                        limit,
                        offset: None,
                        prove,
                    },
                ))
                .await;
            if prove && limit != Some(0) {
                if let Ok(response) = &result {
                    let Some(get_identity_keys_response::Version::V0(v0)) =
                        &response.get_ref().version
                    else {
                        panic!("identity keys response version")
                    };
                    if let Some(get_identity_keys_response_v0::Result::Proof(proof)) = &v0.result {
                        let verified = Drive::verify_identity_keys_by_identity_id(
                            &proof.grovedb_proof,
                            IdentityKeysRequest {
                                identity_id: [1; 32],
                                request_type: KeyRequestType::SpecificKeys(key_ids.clone()),
                                limit: limit.map(|value| {
                                    u16::try_from(value)
                                        .expect("fixture limit fits the Drive request")
                                }),
                                offset: None,
                            },
                            false,
                            false,
                            false,
                            PlatformVersion::latest(),
                        );
                        println!("SpecificKeys {key_ids:?} limit={limit:?} actual Drive verifier: {verified:?}");
                        let (root, partial_identity) =
                            verified.expect("successful specific-key proofs remain verifiable");
                        assert!(
                            partial_identity
                                .is_none_or(|identity| identity.loaded_public_keys.is_empty()),
                            "absence proof must not return public keys"
                        );
                        assert_eq!(
                            root,
                            fixture
                                .platform
                                .drive
                                .grove
                                .root_hash(None, &PlatformVersion::latest().drive.grove_version)
                                .unwrap()
                                .expect("state root")
                        );
                    }
                }
            }
            let expected = if prove && limit == Some(0) {
                Code::InvalidArgument
            } else {
                Code::Ok
            };
            record(
                &format!("SpecificKeys {key_ids:?}, limit={limit:?}"),
                prove,
                result,
                expected,
                &mut failures,
            );
        }
    }
    server.abort();
    assert!(failures.is_empty(), "valid query controls: {failures:?}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_not_classify_supported_unproved_specific_key_caps_as_invalid_input() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    for (key_ids, limit) in [(vec![0], 0), (vec![0, 1], 1)] {
        let response = client
            .get_identity_keys(wire::GetIdentityKeysRequest::from(
                GetIdentityKeysRequestV0 {
                    identity_id: vec![1; 32],
                    request_type: Some(wire::KeyRequestType {
                        request: Some(KeyRequestKind::SpecificKeys(wire::SpecificKeys { key_ids })),
                    }),
                    limit: Some(limit),
                    offset: None,
                    prove: false,
                },
            ))
            .await;
        if let Err(status) = response {
            println!(
                "unproved SpecificKeys return cap={limit}: {:?} {}",
                status.code(),
                status.message()
            );
            assert_ne!(
                status.code(),
                Code::InvalidArgument,
                "a return cap does not require selection cardinality to fit it"
            );
        }
    }
    server.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_preserve_missing_request_version_classification() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    record(
        "contracts missing version",
        false,
        client
            .get_data_contracts(wire::GetDataContractsRequest { version: None })
            .await,
        Code::Unknown,
        &mut failures,
    );
    record(
        "keys missing version",
        false,
        client
            .get_identity_keys(wire::GetIdentityKeysRequest { version: None })
            .await,
        Code::Unknown,
        &mut failures,
    );
    record(
        "epochs missing version",
        false,
        client
            .get_epochs_info(wire::GetEpochsInfoRequest { version: None })
            .await,
        Code::Unknown,
        &mut failures,
    );
    server.abort();
    assert!(failures.is_empty(), "version refusal changed: {failures:?}");
}
