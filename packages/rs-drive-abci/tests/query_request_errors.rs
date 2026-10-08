//! Invalid caller queries must keep healthy nodes available for other requests.
//! The generated tonic server and client run over an in-memory duplex stream.

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_addresses_branch_state_request::GetAddressesBranchStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_infos_request::GetAddressesInfosRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_response::get_data_contract_history_response_v0;
use dapi_grpc::platform::v0::get_data_contracts_by_range_request::GetDataContractsByRangeRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_latest_versions_request::GetDataContractsLatestVersionsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_response::get_data_contracts_response_v0;
use dapi_grpc::platform::v0::get_documents_request::GetDocumentsRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_response::get_epochs_info_response_v0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
use dapi_grpc::platform::v0::get_group_actions_request::GetGroupActionsRequestV0;
use dapi_grpc::platform::v0::get_group_infos_request::GetGroupInfosRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
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
    get_data_contract_history_response, get_data_contracts_request, get_data_contracts_response,
    get_epochs_info_request, get_epochs_info_response, get_identity_keys_response,
    get_path_elements_response,
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
use dpp::serialization::PlatformSerializableWithPlatformVersion;
use dpp::tests::fixtures::get_data_contract_fixture;
use dpp::version::PlatformVersion;
use drive::drive::identity::key::fetch::{IdentityKeysRequest, KeyRequestType};
use drive::drive::shielded::paths::SHIELDED_NOTES_CHUNK_POWER;
use drive::drive::Drive;
use drive::grovedb::Element;
use drive::query::DriveDocumentQuery;
use drive_abci::config::PlatformConfig;
use drive_abci::error::query::QueryError;
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
            .timeout(Duration::from_secs(15))
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
            .timeout(Duration::from_secs(15))
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

fn code<T>(result: Result<T, Status>) -> Code {
    match result {
        Ok(_) => Code::Ok,
        Err(status) => status.code(),
    }
}

macro_rules! assert_matrix_response {
    (get_data_contracts, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_data_contracts_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_data_contracts_response::get_data_contracts_response_v0::Result::DataContracts(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identities_balances, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identities_balances_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identities_balances_response::get_identities_balances_response_v0::Result::IdentitiesBalances(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identities_contract_keys, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identities_contract_keys_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identities_contract_keys_response::get_identities_contract_keys_response_v0::Result::IdentitiesKeys(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_evonodes_proposed_epoch_blocks_by_ids, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_evonodes_proposed_epoch_blocks_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_evonodes_proposed_epoch_blocks_response::get_evonodes_proposed_epoch_blocks_response_v0::Result::EvonodesProposedBlockCountsInfo(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identity_token_balances, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identity_token_balances_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identity_token_balances_response::get_identity_token_balances_response_v0::Result::TokenBalances(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identities_token_balances, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identities_token_balances_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identities_token_balances_response::get_identities_token_balances_response_v0::Result::IdentityTokenBalances(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identity_token_infos, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identity_token_infos_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identity_token_infos_response::get_identity_token_infos_response_v0::Result::TokenInfos(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identities_token_infos, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_identities_token_infos_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_identities_token_infos_response::get_identities_token_infos_response_v0::Result::IdentityTokenInfos(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_token_statuses, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_token_statuses_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_token_statuses_response::get_token_statuses_response_v0::Result::TokenStatuses(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_addresses_infos, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_addresses_infos_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_addresses_infos_response::get_addresses_infos_response_v0::Result::AddressInfoEntries(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_epochs_info, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(wire::get_epochs_info_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 response");
        };
        assert!(!$prove, "this empty selection cannot return a successful proof");
        let Some(wire::get_epochs_info_response::get_epochs_info_response_v0::Result::Epochs(_)) = &v0.result else {
            panic!("an unproved request must return its data variant");
        };
    }};
    (get_identity_keys, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {{
        let Some(get_identity_keys_response::Version::V0(v0)) = &$response.version else {
            panic!("expected the V0 keys response");
        };
        if $prove {
            let Some(get_identity_keys_response_v0::Result::Proof(proof)) = &v0.result else {
                panic!("a proved key response must contain a proof");
            };
            let Some(wire::get_identity_keys_request::Version::V0(request)) = &$request.version else {
                panic!("expected the original V0 keys request");
            };
            let Some(KeyRequestKind::SpecificKeys(ids)) = request.request_type.as_ref().and_then(|kind| kind.request.as_ref()) else {
                panic!("expected the original specific-key selection");
            };
            let (root, identity) = Drive::verify_identity_keys_by_identity_id(
                &proof.grovedb_proof,
                IdentityKeysRequest {
                    identity_id: request.identity_id.clone().try_into().expect("identity identifier"),
                    request_type: KeyRequestType::SpecificKeys(ids.key_ids.clone()),
                    limit: request.limit.map(|limit| u16::try_from(limit).expect("fixture cap")),
                    offset: request.offset.map(|offset| u16::try_from(offset).expect("fixture offset")),
                },
                false, false, false, PlatformVersion::latest(),
            ).expect("the original key selection and cap verify");
            assert_eq!(root, $fixture.platform.drive.grove.root_hash(None, &PlatformVersion::latest().drive.grove_version).value.expect("stored root"));
            assert!(identity.is_none_or(|identity| identity.loaded_public_keys.is_empty()));
        } else {
            let Some(get_identity_keys_response_v0::Result::Keys(_)) = &v0.result else {
                panic!("an unproved key response must contain keys");
            };
        }
    }};
    ($method:ident, $request:expr, $response:expr, $prove:expr, $fixture:expr) => {
        panic!("a successful matrix row needs a concrete variant assertion: {:?}", $response.version);
    };
}

/// What the service answers for each request that is the caller's mistake, in both proof
/// modes. A request without a proof that has an answer (an empty one) keeps it; one GroveDB
/// cannot prove, or that no node can answer, is INVALID_ARGUMENT.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_answer_request_errors_with_invalid_argument() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let id = || vec![1u8; 32];
    let mut wrong = Vec::new();
    let mut expect = |name: &str, prove: bool, expected: Code, actual: Code| {
        if actual != expected {
            wrong.push(format!(
                "{name} prove={prove}: expected {expected:?}, got {actual:?}"
            ));
        }
    };

    for prove in [false, true] {
        // An empty selection: answered empty without a proof, refused with one.
        let empty = if prove {
            Code::InvalidArgument
        } else {
            Code::Ok
        };
        macro_rules! check {
            ($name:expr, $expected:expr, $method:ident, $request_type:ident, $request:expr) => {{
                let request: wire::$request_type = $request.into();
                let result = client.$method(request.clone()).await;
                if $expected == Code::Ok {
                    if let Ok(response) = &result {
                        assert_matrix_response!(
                            $method,
                            &request,
                            response.get_ref(),
                            prove,
                            &fixture
                        );
                    }
                }
                if let Err(status) = &result {
                    println!(
                        "{} prove={prove}: {:?} {}",
                        $name,
                        status.code(),
                        status.message()
                    );
                    if $expected == Code::InvalidArgument
                        && ($name.contains("limit") || $name.contains("count"))
                    {
                        assert!(
                            status.message().contains("limit")
                                || status.message().contains("count"),
                            "the refusal must identify the cap: {status}"
                        );
                    }
                }
                expect($name, prove, $expected, code(result));
            }};
        }

        check!(
            "data contracts, no ids",
            empty,
            get_data_contracts,
            GetDataContractsRequest,
            GetDataContractsRequestV0 { ids: vec![], prove }
        );
        check!(
            "identities balances, no ids",
            empty,
            get_identities_balances,
            GetIdentitiesBalancesRequest,
            GetIdentitiesBalancesRequestV0 { ids: vec![], prove }
        );
        check!(
            "identities contract keys, no ids",
            empty,
            get_identities_contract_keys,
            GetIdentitiesContractKeysRequest,
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![],
                contract_id: id(),
                document_type_name: None,
                purposes: vec![0],
                prove,
            }
        );
        check!(
            "identities contract keys, no purposes",
            empty,
            get_identities_contract_keys,
            GetIdentitiesContractKeysRequest,
            GetIdentitiesContractKeysRequestV0 {
                identities_ids: vec![id()],
                contract_id: id(),
                document_type_name: None,
                purposes: vec![],
                prove,
            }
        );
        check!(
            "evonode blocks by ids, no ids",
            empty,
            get_evonodes_proposed_epoch_blocks_by_ids,
            GetEvonodesProposedEpochBlocksByIdsRequest,
            GetEvonodesProposedEpochBlocksByIdsRequestV0 {
                epoch: None,
                ids: vec![],
                prove,
            }
        );
        check!(
            "identity token balances, no token ids",
            empty,
            get_identity_token_balances,
            GetIdentityTokenBalancesRequest,
            GetIdentityTokenBalancesRequestV0 {
                identity_id: id(),
                token_ids: vec![],
                prove,
            }
        );
        check!(
            "identities token balances, no identity ids",
            empty,
            get_identities_token_balances,
            GetIdentitiesTokenBalancesRequest,
            GetIdentitiesTokenBalancesRequestV0 {
                token_id: id(),
                identity_ids: vec![],
                prove,
            }
        );
        check!(
            "identity token infos, no token ids",
            empty,
            get_identity_token_infos,
            GetIdentityTokenInfosRequest,
            GetIdentityTokenInfosRequestV0 {
                identity_id: id(),
                token_ids: vec![],
                prove,
            }
        );
        check!(
            "identities token infos, no identity ids",
            empty,
            get_identities_token_infos,
            GetIdentitiesTokenInfosRequest,
            GetIdentitiesTokenInfosRequestV0 {
                token_id: id(),
                identity_ids: vec![],
                prove,
            }
        );
        check!(
            "token statuses, no token ids",
            empty,
            get_token_statuses,
            GetTokenStatusesRequest,
            GetTokenStatusesRequestV0 {
                token_ids: vec![],
                prove,
            }
        );
        check!(
            "addresses infos, no addresses",
            empty,
            get_addresses_infos,
            GetAddressesInfosRequest,
            GetAddressesInfosRequestV0 {
                addresses: vec![],
                prove,
            }
        );
        check!(
            "identity keys, limit 0",
            empty,
            get_identity_keys,
            GetIdentityKeysRequest,
            GetIdentityKeysRequestV0 {
                identity_id: id(),
                request_type: Some(wire::KeyRequestType {
                    request: Some(KeyRequestKind::AllKeys(wire::AllKeys {})),
                }),
                limit: Some(0),
                offset: None,
                prove,
            }
        );
        check!(
            "epochs info, count 0",
            empty,
            get_epochs_info,
            GetEpochsInfoRequest,
            wire::GetEpochsInfoRequest {
                version: Some(get_epochs_info_request::Version::V0(
                    GetEpochsInfoRequestV0 {
                        start_epoch: None,
                        count: 0,
                        ascending: true,
                        prove,
                    }
                )),
            }
        );

        // A specific-keys request whose limit is below its number of ids: served either way.
        check!(
            "identity keys, two ids and limit 1",
            Code::Ok,
            get_identity_keys,
            GetIdentityKeysRequest,
            GetIdentityKeysRequestV0 {
                identity_id: id(),
                request_type: Some(wire::KeyRequestType {
                    request: Some(KeyRequestKind::SpecificKeys(wire::SpecificKeys {
                        key_ids: vec![0, 1],
                    })),
                }),
                limit: Some(1),
                offset: None,
                prove,
            }
        );

        // No node can answer these, with or without a proof.
        let refused = Code::InvalidArgument;
        check!(
            "contract history, limit 0",
            refused,
            get_data_contract_history,
            GetDataContractHistoryRequest,
            GetDataContractHistoryRequestV0 {
                id: id(),
                limit: Some(0),
                offset: None,
                start_at_ms: 0,
                prove,
            }
        );
        check!(
            "contract history, limit 11",
            refused,
            get_data_contract_history,
            GetDataContractHistoryRequest,
            GetDataContractHistoryRequestV0 {
                id: id(),
                limit: Some(11),
                offset: None,
                start_at_ms: 0,
                prove,
            }
        );
        check!(
            "protocol version upgrade vote status, count 0",
            refused,
            get_protocol_version_upgrade_vote_status,
            GetProtocolVersionUpgradeVoteStatusRequest,
            GetProtocolVersionUpgradeVoteStatusRequestV0 {
                start_pro_tx_hash: vec![],
                count: 0,
                prove,
            }
        );
        check!(
            "evonode blocks by range, limit 0",
            refused,
            get_evonodes_proposed_epoch_blocks_by_range,
            GetEvonodesProposedEpochBlocksByRangeRequest,
            GetEvonodesProposedEpochBlocksByRangeRequestV0 {
                epoch: None,
                limit: Some(0),
                start: None,
                prove,
            }
        );
        check!(
            "contested resource identity votes, limit 0",
            refused,
            get_contested_resource_identity_votes,
            GetContestedResourceIdentityVotesRequest,
            GetContestedResourceIdentityVotesRequestV0 {
                identity_id: id(),
                limit: Some(0),
                offset: None,
                order_ascending: true,
                start_at_vote_poll_id_info: None,
                prove,
            }
        );
        check!(
            "token pre-programmed distributions, limit 0",
            refused,
            get_token_pre_programmed_distributions,
            GetTokenPreProgrammedDistributionsRequest,
            GetTokenPreProgrammedDistributionsRequestV0 {
                token_id: id(),
                start_at_info: None,
                limit: Some(0),
                prove,
            }
        );
        check!(
            "group infos, count 0",
            refused,
            get_group_infos,
            GetGroupInfosRequest,
            GetGroupInfosRequestV0 {
                contract_id: id(),
                start_at_group_contract_position: None,
                count: Some(0),
                prove,
            }
        );
        check!(
            "group actions, count 0",
            refused,
            get_group_actions,
            GetGroupActionsRequest,
            GetGroupActionsRequestV0 {
                contract_id: id(),
                group_contract_position: 0,
                status: 0,
                start_at_action_id: None,
                count: Some(0),
                prove,
            }
        );
    }

    for depth in [0, 255, 263] {
        expect(
            &format!("addresses branch state, depth {depth}"),
            true,
            Code::InvalidArgument,
            code(
                client
                    .get_addresses_branch_state(Request::new(
                        GetAddressesBranchStateRequestV0 {
                            key: vec![],
                            depth,
                            checkpoint_height: 0,
                        }
                        .into(),
                    ))
                    .await,
            ),
        );
    }

    // A request with no version may be a newer one this node cannot decode, which another
    // node may serve: it stays retryable.
    for (name, actual) in [
        (
            "data contracts, no version",
            code(
                client
                    .get_data_contracts(Request::new(wire::GetDataContractsRequest {
                        version: None,
                    }))
                    .await,
            ),
        ),
        (
            "identity keys, no version",
            code(
                client
                    .get_identity_keys(Request::new(wire::GetIdentityKeysRequest { version: None }))
                    .await,
            ),
        ),
        (
            "epochs info, no version",
            code(
                client
                    .get_epochs_info(Request::new(wire::GetEpochsInfoRequest { version: None }))
                    .await,
            ),
        ),
    ] {
        expect(name, false, Code::Unknown, actual);
    }

    server.abort();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_retain_existing_request_error_and_valid_batch_contracts() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let mut failures = Vec::new();
    let version = PlatformVersion::latest();
    let stored_root = fixture
        .platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .expect("stored root");
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
        let requested_id = [1; 32];
        let response = client
            .get_data_contracts(wire::GetDataContractsRequest::from(
                GetDataContractsRequestV0 {
                    ids: vec![requested_id.to_vec()],
                    prove,
                },
            ))
            .await
            .expect("nonempty contract batch succeeds")
            .into_inner();
        let Some(get_data_contracts_response::Version::V0(v0)) = response.version else {
            panic!("expected the V0 contract batch response");
        };
        if prove {
            let Some(get_data_contracts_response_v0::Result::Proof(proof)) = v0.result else {
                panic!("a proved contract batch must contain a proof");
            };
            let (root, contracts) =
                Drive::verify_contracts(&proof.grovedb_proof, false, &[requested_id], version)
                    .expect("the original contract batch query verifies");
            assert_eq!(root, stored_root);
            assert_eq!(contracts.len(), 1);
            assert!(matches!(contracts.get(&requested_id), Some(None)));
        } else {
            let Some(get_data_contracts_response_v0::Result::DataContracts(contracts)) = v0.result
            else {
                panic!("an unproved contract batch must contain contract entries");
            };
            assert_eq!(contracts.data_contract_entries.len(), 1);
            assert_eq!(contracts.data_contract_entries[0].identifier, requested_id);
            assert!(contracts.data_contract_entries[0].data_contract.is_none());
        }
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

// This raw adapter leaves names_nothing at its default so empty proofs reach the
// server instead of being refused by the generated request's client preflight.
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
    let version = PlatformVersion::latest();
    let stored_root = fixture
        .platform
        .drive
        .grove
        .root_hash(None, &version.drive.grove_version)
        .value
        .expect("stored root");
    let contract = fixture
        .platform
        .drive
        .fetch_contract(contract_id.to_buffer(), None, None, None, version)
        .value
        .expect("contract fetch")
        .expect("stored contract");
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
        let Some(wire::get_documents_response::Version::V0(v0)) = &zero.version else {
            panic!("expected the V0 documents response");
        };
        if prove {
            let Some(wire::get_documents_response::get_documents_response_v0::Result::Proof(proof)) =
                &v0.result
            else {
                panic!("a proved document default must contain a proof");
            };
            let query = DriveDocumentQuery::all_items_query(
                &contract.contract,
                contract
                    .contract
                    .document_type_for_name(&document_type)
                    .expect("document type"),
                Some(100),
            );
            let (root, documents) = query
                .verify_proof(&proof.grovedb_proof, version)
                .expect("original default document query verifies");
            assert_eq!(root, stored_root);
            assert!(documents.is_empty());
        } else {
            let Some(wire::get_documents_response::get_documents_response_v0::Result::Documents(
                documents,
            )) = &v0.result
            else {
                panic!("an unproved document default must contain documents");
            };
            assert!(documents.documents.is_empty());
        }
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
        let Some(wire::get_shielded_encrypted_notes_response::Version::V0(v0)) = &zero.version
        else {
            panic!("expected the V0 shielded notes response");
        };
        if prove {
            let Some(wire::get_shielded_encrypted_notes_response::get_shielded_encrypted_notes_response_v0::Result::Proof(proof)) = &v0.result else {
                panic!("a proved shielded default must contain a proof");
            };
            let max_notes = (version.drive_abci.query.shielded_queries.max_query_chunks as u32)
                .saturating_mul(1 << SHIELDED_NOTES_CHUNK_POWER);
            let (root, notes, total_count) = Drive::verify_shielded_encrypted_notes(
                &proof.grovedb_proof,
                0,
                0,
                max_notes,
                false,
                version,
            )
            .expect("original default shielded query verifies");
            assert_eq!(root, stored_root);
            assert!(notes.is_empty());
            assert_eq!(total_count, 0);
        } else {
            let Some(wire::get_shielded_encrypted_notes_response::get_shielded_encrypted_notes_response_v0::Result::EncryptedNotes(notes)) = &v0.result else {
                panic!("an unproved shielded default must contain encrypted notes");
            };
            assert!(notes.entries.is_empty());
        }
        for limit in [None, Some(1)] {
            let drive_limit = limit
                .map(|value| u16::try_from(value).expect("fixture limit fits the Drive request"));
            let history = client
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
                .expect("positive and absent historical contract caps remain valid")
                .into_inner();
            let Some(get_data_contract_history_response::Version::V0(v0)) = history.version else {
                panic!("expected the V0 contract history response");
            };
            if prove {
                let Some(get_data_contract_history_response_v0::Result::Proof(proof)) = v0.result
                else {
                    panic!("a proved contract history must contain a proof");
                };
                let (root, history) = Drive::verify_contract_history(
                    &proof.grovedb_proof,
                    contract_id.to_buffer(),
                    0,
                    drive_limit,
                    None,
                    version,
                )
                .expect("the original optional-cap contract history query verifies");
                assert_eq!(root, stored_root);
                let history = history.expect("stored contract has history");
                assert_eq!(history.len(), 1);
                assert_eq!(history.get(&1000), Some(&contract.contract));
            } else {
                let Some(get_data_contract_history_response_v0::Result::DataContractHistory(
                    history,
                )) = v0.result
                else {
                    panic!("an unproved contract history must contain history entries");
                };
                assert_eq!(history.data_contract_entries.len(), 1);
                assert_eq!(history.data_contract_entries[0].date, 1000);
                assert_eq!(
                    history.data_contract_entries[0].value,
                    contract
                        .contract
                        .serialize_to_bytes_with_platform_version(version)
                        .expect("serialize the stored contract")
                );
            }
            let keys = client
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
                .expect("positive and absent key caps remain valid")
                .into_inner();
            let Some(get_identity_keys_response::Version::V0(v0)) = keys.version else {
                panic!("expected the V0 identity keys response");
            };
            if prove {
                let Some(get_identity_keys_response_v0::Result::Proof(proof)) = v0.result else {
                    panic!("a proved AllKeys selection must contain a proof");
                };
                let (root, identity) = Drive::verify_identity_keys_by_identity_id(
                    &proof.grovedb_proof,
                    IdentityKeysRequest {
                        identity_id: [1; 32],
                        request_type: KeyRequestType::AllKeys,
                        limit: drive_limit,
                        offset: None,
                    },
                    false,
                    false,
                    false,
                    version,
                )
                .expect("the original optional-cap AllKeys query verifies");
                assert_eq!(root, stored_root);
                assert!(identity.is_none_or(|identity| identity.loaded_public_keys.is_empty()));
            } else {
                let Some(get_identity_keys_response_v0::Result::Keys(keys)) = v0.result else {
                    panic!("an unproved AllKeys selection must contain keys");
                };
                assert!(keys.keys_bytes.is_empty());
            }
        }
        let epochs = client
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
            .await
            .expect("a positive epoch count remains valid")
            .into_inner();
        let Some(get_epochs_info_response::Version::V0(v0)) = epochs.version else {
            panic!("expected the V0 epoch response");
        };
        if prove {
            let Some(get_epochs_info_response_v0::Result::Proof(proof)) = v0.result else {
                panic!("a proved epoch query must contain a proof");
            };
            let current_epoch = u16::try_from(v0.metadata.expect("epoch metadata").epoch)
                .expect("fixture epoch fits the Drive request");
            let (root, epochs) = Drive::verify_epoch_infos(
                &proof.grovedb_proof,
                current_epoch,
                Some(0),
                1,
                true,
                version,
            )
            .expect("the original positive-count epoch query verifies");
            assert_eq!(root, stored_root);
            assert!(epochs.is_empty());
        } else {
            let Some(get_epochs_info_response_v0::Result::Epochs(epochs)) = v0.result else {
                panic!("an unproved epoch query must contain epochs");
            };
            assert!(epochs.epoch_infos.is_empty());
        }
        let elements = client
            .get_path_elements(wire::GetPathElementsRequest::from(
                GetPathElementsRequestV0 {
                    path: vec![],
                    keys: vec![vec![96]],
                    prove,
                },
            ))
            .await
            .expect("a root-path key selection remains valid")
            .into_inner();
        let Some(get_path_elements_response::Version::V0(v0)) = elements.version else {
            panic!("expected the V0 path response");
        };
        if prove {
            let Some(get_path_elements_response_v0::Result::Proof(proof)) = v0.result else {
                panic!("a proved root-path key selection must contain a proof");
            };
            let (root, elements) =
                Drive::verify_elements(&proof.grovedb_proof, vec![], vec![vec![96]], version)
                    .expect("the original root-path key query verifies");
            assert_eq!(root, stored_root);
            assert_eq!(elements.len(), 1);
            assert_eq!(
                elements.get(&vec![96]),
                Some(&Some(Element::empty_sum_tree()))
            );
        } else {
            let Some(get_path_elements_response_v0::Result::Elements(elements)) = v0.result else {
                panic!("an unproved root-path key selection must contain elements");
            };
            assert_eq!(
                elements.elements,
                vec![Element::empty_sum_tree()
                    .serialize(&version.drive.grove_version)
                    .expect("serialize the initial balances tree")]
            );
        }
        let mut selections = vec![(vec![], None), (vec![], Some(0)), (vec![0, 0], Some(1))];
        selections.extend([(vec![0], Some(0)), (vec![0, 1], Some(1))]);
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
                    let Some(get_identity_keys_response_v0::Result::Proof(proof)) = &v0.result
                    else {
                        panic!("proved identity-key response must contain a proof")
                    };
                    let verified = Drive::verify_identity_keys_by_identity_id(
                        &proof.grovedb_proof,
                        IdentityKeysRequest {
                            identity_id: [1; 32],
                            request_type: KeyRequestType::SpecificKeys(key_ids.clone()),
                            limit: limit.map(|value| {
                                u16::try_from(value).expect("fixture limit fits the Drive request")
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
            if !prove {
                let response = result.as_ref().expect("unproved key selection succeeds");
                let Some(get_identity_keys_response::Version::V0(v0)) = &response.get_ref().version
                else {
                    panic!("identity keys response version");
                };
                let Some(get_identity_keys_response_v0::Result::Keys(keys)) = &v0.result else {
                    panic!("an unproved key selection must contain keys");
                };
                assert!(
                    keys.keys_bytes.is_empty(),
                    "the fixture identity has no keys"
                );
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
async fn should_return_a_verifiable_proof_for_an_empty_root_path_selection() {
    let fixture = QueryFixture::new();
    let (mut client, server) = fixture.client();
    let response = client
        .get_path_elements(wire::GetPathElementsRequest::from(
            GetPathElementsRequestV0 {
                path: vec![],
                keys: vec![],
                prove: true,
            },
        ))
        .await
        .expect("a root-path selection is supported")
        .into_inner();
    let Some(get_path_elements_response::Version::V0(v0)) = response.version else {
        panic!("expected the V0 path response");
    };
    let Some(get_path_elements_response_v0::Result::Proof(proof)) = v0.result else {
        panic!("a proved root-path response must contain a proof");
    };
    let (root, elements) = Drive::verify_elements(
        &proof.grovedb_proof,
        vec![],
        vec![],
        PlatformVersion::latest(),
    )
    .expect("the original empty root-path query verifies");
    assert!(elements.is_empty());
    assert_eq!(
        root,
        fixture
            .platform
            .drive
            .grove
            .root_hash(None, &PlatformVersion::latest().drive.grove_version)
            .value
            .expect("stored root")
    );
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_deliver_bounded_oversized_statuses_over_real_grpc() {
    let fixture = QueryFixture::new();
    let (contract_id, _) = fixture.store_contract();
    let mut failures = Vec::new();
    // "document type " occupies 14 bytes; the é therefore crosses byte 1024.
    for (document_type, retained_bytes) in [
        ("x".repeat(100_000), 1024),
        (format!("{}{}", "x".repeat(1009), "é%".repeat(40_000)), 1023),
        ("%".repeat(100_000), 1024),
    ] {
        let (mut client, server) = fixture.client();
        let request = wire::GetDocumentsRequest::from(GetDocumentsRequestV0 {
            data_contract_id: contract_id.to_vec(),
            document_type,
            r#where: vec![],
            order_by: vec![],
            limit: 0,
            start: None,
            prove: false,
        });
        let original = fixture
            .platform
            .query_documents(
                request.clone(),
                &fixture.platform.state.load_full(),
                PlatformVersion::latest(),
            )
            .expect("the handler returns a validation result");
        let QueryError::InvalidArgument(original_message) =
            original.errors.first().expect("unknown document type")
        else {
            panic!("expected the original invalid-argument message");
        };
        assert!(original_message.len() > 16 * 1024);
        assert!(original_message.is_char_boundary(retained_bytes));
        if retained_bytes == 1023 {
            assert!(!original_message.is_char_boundary(1024));
        }
        let expected = format!(
            "{}... ({} bytes truncated)",
            &original_message[..retained_bytes],
            original_message.len() - retained_bytes
        );
        let status = tokio::time::timeout(Duration::from_secs(5), client.get_documents(request))
            .await
            .expect("bounded status transport")
            .expect_err("unknown document type");
        println!(
            "oversized status over tonic: code={:?}, message_bytes={}, message_prefix={:?}",
            status.code(),
            status.message().len(),
            status.message().chars().take(80).collect::<String>()
        );
        if status.code() != Code::InvalidArgument || status.message() != expected {
            failures.push(format!("prefix bytes={retained_bytes}: expected InvalidArgument and {expected:?}, got {status:?}"));
        }
        server.abort();
    }
    assert!(
        failures.is_empty(),
        "oversized status transport: {failures:?}"
    );
}
