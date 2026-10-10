//! A query request that is the caller's mistake gets INVALID_ARGUMENT from the node, so the
//! client neither bans the node nor sends the request on to the next one. The real
//! `QueryService` answers here, called in-process; the client's ban logic only ever sees the
//! status it returns.

use dapi_grpc::mock::Mockable;
use dapi_grpc::platform::v0::get_addresses_branch_state_request::GetAddressesBranchStateRequestV0;
use dapi_grpc::platform::v0::get_addresses_infos_request::GetAddressesInfosRequestV0;
use dapi_grpc::platform::v0::get_contested_resource_identity_votes_request::GetContestedResourceIdentityVotesRequestV0;
use dapi_grpc::platform::v0::get_data_contract_history_request::GetDataContractHistoryRequestV0;
use dapi_grpc::platform::v0::get_data_contracts_request::GetDataContractsRequestV0;
use dapi_grpc::platform::v0::get_epochs_info_request;
use dapi_grpc::platform::v0::get_epochs_info_request::GetEpochsInfoRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_ids_request::GetEvonodesProposedEpochBlocksByIdsRequestV0;
use dapi_grpc::platform::v0::get_evonodes_proposed_epoch_blocks_by_range_request::GetEvonodesProposedEpochBlocksByRangeRequestV0;
use dapi_grpc::platform::v0::get_group_actions_request::GetGroupActionsRequestV0;
use dapi_grpc::platform::v0::get_group_infos_request::GetGroupInfosRequestV0;
use dapi_grpc::platform::v0::get_identities_balances_request::GetIdentitiesBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_contract_keys_request::GetIdentitiesContractKeysRequestV0;
use dapi_grpc::platform::v0::get_identities_token_balances_request::GetIdentitiesTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identities_token_infos_request::GetIdentitiesTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_identity_keys_request::GetIdentityKeysRequestV0;
use dapi_grpc::platform::v0::get_identity_token_balances_request::GetIdentityTokenBalancesRequestV0;
use dapi_grpc::platform::v0::get_identity_token_infos_request::GetIdentityTokenInfosRequestV0;
use dapi_grpc::platform::v0::get_protocol_version_upgrade_vote_status_request::GetProtocolVersionUpgradeVoteStatusRequestV0;
use dapi_grpc::platform::v0::get_token_pre_programmed_distributions_request::GetTokenPreProgrammedDistributionsRequestV0;
use dapi_grpc::platform::v0::get_token_statuses_request::GetTokenStatusesRequestV0;
use dapi_grpc::platform::v0::key_request_type::Request as KeyRequest;
use dapi_grpc::platform::v0::platform_server::Platform as PlatformService;
use dapi_grpc::platform::v0::{
    AllKeys, GetDataContractsRequest, GetDataContractsResponse, GetEpochsInfoRequest,
    GetIdentityKeysRequest, KeyRequestType, SpecificKeys,
};
use dapi_grpc::tonic::{Code, Request, Status};
use dpp::dashcore::Network;
use dpp::version::PlatformVersion;
use drive_abci::config::PlatformConfig;
use drive_abci::query::QueryService;
use drive_abci::test::helpers::setup::TestPlatformBuilder;
use rs_dapi_client::transport::{
    AppliedRequestSettings, BoxFuture, TransportClient, TransportError, TransportRequest,
};
use rs_dapi_client::{
    AddressList, ConnectionPool, DapiClient, DapiClientError, DapiRequestExecutor, RequestSettings,
    Uri,
};
use std::fmt::{self, Debug};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

/// A node's query service over a fresh, initialized state.
struct Node {
    service: Arc<QueryService>,
    _directory: TempDir,
}

impl Node {
    fn new() -> Self {
        let mut config = PlatformConfig::default_for_network(Network::Testnet);
        // Never reached: no query here asks Core anything.
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
            .expect("expected the initial state structure");
        Self {
            service: Arc::new(QueryService::new(Arc::new(temporary.platform))),
            _directory: temporary.tempdir,
        }
    }
}

fn code<T>(result: Result<T, Status>) -> Code {
    match result {
        Ok(_) => Code::Ok,
        Err(status) => status.code(),
    }
}

/// What the service answers for each request that is the caller's mistake, in both proof
/// modes. A request without a proof that has an answer (an empty one) keeps it; one GroveDB
/// cannot prove, or that no node can answer, is INVALID_ARGUMENT.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_answer_request_errors_with_invalid_argument() {
    let node = Node::new();
    let service = node.service.as_ref();
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
            ($name:expr, $expected:expr, $method:ident, $request:expr) => {
                expect(
                    $name,
                    prove,
                    $expected,
                    code(service.$method(Request::new($request.into())).await),
                )
            };
        }

        check!(
            "data contracts, no ids",
            empty,
            get_data_contracts,
            GetDataContractsRequestV0 { ids: vec![], prove }
        );
        check!(
            "identities balances, no ids",
            empty,
            get_identities_balances,
            GetIdentitiesBalancesRequestV0 { ids: vec![], prove }
        );
        check!(
            "identities contract keys, no ids",
            empty,
            get_identities_contract_keys,
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
            GetTokenStatusesRequestV0 {
                token_ids: vec![],
                prove,
            }
        );
        check!(
            "addresses infos, no addresses",
            empty,
            get_addresses_infos,
            GetAddressesInfosRequestV0 {
                addresses: vec![],
                prove,
            }
        );
        check!(
            "identity keys, limit 0",
            empty,
            get_identity_keys,
            GetIdentityKeysRequestV0 {
                identity_id: id(),
                request_type: Some(KeyRequestType {
                    request: Some(KeyRequest::AllKeys(AllKeys {})),
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
            GetEpochsInfoRequest {
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
            GetIdentityKeysRequestV0 {
                identity_id: id(),
                request_type: Some(KeyRequestType {
                    request: Some(KeyRequest::SpecificKeys(SpecificKeys {
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
            GetDataContractHistoryRequestV0 {
                id: id(),
                limit: Some(0),
                offset: None,
                start_at_ms: 0,
                prove,
            }
        );
        check!(
            "protocol version upgrade vote status, count 0",
            refused,
            get_protocol_version_upgrade_vote_status,
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

    expect(
        "addresses branch state, depth 0",
        true,
        Code::InvalidArgument,
        code(
            service
                .get_addresses_branch_state(Request::new(
                    GetAddressesBranchStateRequestV0 {
                        key: vec![],
                        depth: 0,
                        checkpoint_height: 0,
                    }
                    .into(),
                ))
                .await,
        ),
    );

    // A request with no version may be a newer one this node cannot decode, which another
    // node may serve: it stays retryable.
    for (name, actual) in [
        (
            "data contracts, no version",
            code(
                service
                    .get_data_contracts(Request::new(GetDataContractsRequest { version: None }))
                    .await,
            ),
        ),
        (
            "identity keys, no version",
            code(
                service
                    .get_identity_keys(Request::new(GetIdentityKeysRequest { version: None }))
                    .await,
            ),
        ),
        (
            "epochs info, no version",
            code(
                service
                    .get_epochs_info(Request::new(GetEpochsInfoRequest { version: None }))
                    .await,
            ),
        ),
    ] {
        expect(name, false, Code::Unknown, actual);
    }

    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Tells nodes apart by address; every node is the same in-process service.
struct InProcessNode {
    uri: Uri,
}

impl TransportClient for InProcessNode {
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

/// A data contracts request answered by the node's service, or by a fault on the first
/// attempt.
#[derive(Clone)]
struct ServedContractsRequest {
    service: Arc<QueryService>,
    request: GetDataContractsRequest,
    first_attempt_fault: Option<Code>,
    attempts: Arc<Mutex<Vec<Uri>>>,
}

impl Debug for ServedContractsRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ServedContractsRequest")
            .field("request", &self.request)
            .finish()
    }
}

impl Mockable for ServedContractsRequest {}

impl TransportRequest for ServedContractsRequest {
    type Client = InProcessNode;
    type Response = GetDataContractsResponse;
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
            let mut attempts = self.attempts.lock().expect("attempts");
            attempts.push(client.uri.clone());
            attempts.len() == 1
        };
        Box::pin(async move {
            if let Some(code) = self.first_attempt_fault.filter(|_| first_attempt) {
                return Err(TransportError::Grpc(Status::new(code, "node fault")));
            }
            self.service
                .get_data_contracts(Request::new(self.request))
                .await
                .map(|response| response.into_inner())
                .map_err(TransportError::Grpc)
        })
    }
}

fn thirteen_nodes() -> AddressList {
    let mut addresses = AddressList::with_settings(Duration::from_secs(3600));
    for port in 20000..20013 {
        addresses.add(format!("http://127.0.0.1:{port}").parse().expect("address"));
    }
    addresses
}

fn grpc_code(error: &DapiClientError) -> Option<Code> {
    match error {
        DapiClientError::Transport(TransportError::Grpc(status)) => Some(status.code()),
        _ => None,
    }
}

/// Three requests for a proof of no data contracts once got all 13 devnet nodes banned.
/// Each now reaches one node, which refuses it, and no node is banned.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_keep_every_node_after_three_empty_contract_proof_requests() {
    let node = Node::new();
    let request = ServedContractsRequest {
        service: Arc::clone(&node.service),
        request: GetDataContractsRequestV0 {
            ids: vec![],
            prove: true,
        }
        .into(),
        first_attempt_fault: None,
        attempts: Default::default(),
    };
    let client = DapiClient::new(
        thirteen_nodes(),
        RequestSettings {
            retries: Some(5),
            ..RequestSettings::default()
        },
    );

    for call in 1..=3 {
        let error = client
            .execute(request.clone(), RequestSettings::default())
            .await
            .expect_err("an empty proof request is refused");
        assert_eq!(grpc_code(&error.inner), Some(Code::InvalidArgument));
        assert_eq!(
            error.retries, 0,
            "call {call} must not move to another node"
        );
        assert_eq!(request.attempts.lock().expect("attempts").len(), call);
        assert!(
            client
                .address_list()
                .ban_info()
                .iter()
                .all(|ban| !ban.banned),
            "call {call} banned a node"
        );
    }
}

/// A fault of the node itself still bans it and moves the request to the next node, which
/// the service answers.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn should_still_ban_and_move_on_when_the_node_fails() {
    let node = Node::new();
    for fault in [Code::Internal, Code::Unavailable] {
        let request = ServedContractsRequest {
            service: Arc::clone(&node.service),
            request: GetDataContractsRequestV0 {
                ids: vec![vec![1; 32]],
                prove: false,
            }
            .into(),
            first_attempt_fault: Some(fault),
            attempts: Default::default(),
        };
        let client = DapiClient::new(
            thirteen_nodes(),
            RequestSettings {
                retries: Some(5),
                ..RequestSettings::default()
            },
        );

        let response = client
            .execute(request.clone(), RequestSettings::default())
            .await
            .expect("the next node answers");

        let attempts = request.attempts.lock().expect("attempts").clone();
        assert_eq!(attempts.len(), 2, "{fault:?} moves the request on once");
        let failed = attempts[0].clone().try_into().expect("address");
        assert!(client.address_list().is_banned(&failed), "{fault:?}");
        assert_eq!(response.retries, 1);
        assert_eq!(response.address.uri(), &attempts[1]);
    }
}
