use crate::error::query::QueryError;
use crate::error::Error;
use crate::metrics::{abci_response_code_metric_label, query_duration_metric};
use crate::platform_types::platform::Platform;
use crate::platform_types::platform_state::v0::PlatformStateV0Methods;
use crate::platform_types::platform_state::PlatformState;
use crate::query::QueryValidationResult;
use crate::rpc::core::DefaultCoreRPC;
use crate::utils::spawn_blocking_task_with_name_if_supported;
use async_trait::async_trait;
use dapi_grpc::drive::v0::drive_internal_server::DriveInternal;
use dapi_grpc::drive::v0::{GetProofsRequest, GetProofsResponse};
use dapi_grpc::platform::v0::platform_events_response::PlatformEventsResponseV0;
use dapi_grpc::platform::v0::platform_server::Platform as PlatformService;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, GetConsensusParamsRequest,
    GetConsensusParamsResponse, GetContestedResourceIdentityVotesRequest,
    GetContestedResourceIdentityVotesResponse, GetContestedResourceVoteStateRequest,
    GetContestedResourceVoteStateResponse, GetContestedResourceVotersForIdentityRequest,
    GetContestedResourceVotersForIdentityResponse, GetContestedResourcesRequest,
    GetContestedResourcesResponse, GetCurrentQuorumsInfoRequest, GetCurrentQuorumsInfoResponse,
    GetDataContractHistoryRequest, GetDataContractHistoryResponse, GetDataContractRequest,
    GetDataContractResponse, GetDataContractsRequest, GetDataContractsResponse,
    GetDocumentsRequest, GetDocumentsResponse, GetEpochsInfoRequest, GetEpochsInfoResponse,
    GetEvonodesProposedEpochBlocksByIdsRequest, GetEvonodesProposedEpochBlocksByRangeRequest,
    GetEvonodesProposedEpochBlocksResponse, GetFinalizedEpochInfosRequest,
    GetFinalizedEpochInfosResponse, GetGroupActionSignersRequest, GetGroupActionSignersResponse,
    GetGroupActionsRequest, GetGroupActionsResponse, GetGroupInfoRequest, GetGroupInfoResponse,
    GetGroupInfosRequest, GetGroupInfosResponse, GetIdentitiesBalancesRequest,
    GetIdentitiesBalancesResponse, GetIdentitiesContractKeysRequest,
    GetIdentitiesContractKeysResponse, GetIdentitiesTokenBalancesRequest,
    GetIdentitiesTokenBalancesResponse, GetIdentitiesTokenInfosRequest,
    GetIdentitiesTokenInfosResponse, GetIdentityBalanceAndRevisionRequest,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceRequest, GetIdentityBalanceResponse,
    GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse,
    GetIdentityByPublicKeyHashRequest, GetIdentityByPublicKeyHashResponse,
    GetIdentityContractNonceRequest, GetIdentityContractNonceResponse, GetIdentityKeysRequest,
    GetIdentityKeysResponse, GetIdentityNonceRequest, GetIdentityNonceResponse, GetIdentityRequest,
    GetIdentityResponse, GetIdentityTokenBalancesRequest, GetIdentityTokenBalancesResponse,
    GetIdentityTokenInfosRequest, GetIdentityTokenInfosResponse, GetPathElementsRequest,
    GetPathElementsResponse, GetPrefundedSpecializedBalanceRequest,
    GetPrefundedSpecializedBalanceResponse, GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeStateResponse, GetProtocolVersionUpgradeVoteStatusRequest,
    GetProtocolVersionUpgradeVoteStatusResponse, GetStatusRequest, GetStatusResponse,
    GetTokenContractInfoRequest, GetTokenContractInfoResponse, GetTokenDirectPurchasePricesRequest,
    GetTokenDirectPurchasePricesResponse, GetTokenPerpetualDistributionLastClaimRequest,
    GetTokenPerpetualDistributionLastClaimResponse, GetTokenPreProgrammedDistributionsRequest,
    GetTokenPreProgrammedDistributionsResponse, GetTokenStatusesRequest, GetTokenStatusesResponse,
    GetTokenTotalSupplyRequest, GetTokenTotalSupplyResponse, GetTotalCreditsInPlatformRequest,
    GetTotalCreditsInPlatformResponse, GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse,
    PlatformEventV0 as PlatformEvent, PlatformEventsCommand, PlatformEventsResponse,
    WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::Streaming;
use dapi_grpc::tonic::{Code, Request, Response, Status};
use dash_event_bus::event_bus::{EventBus, Filter as EventBusFilter, SubscriptionHandle};
use dash_event_bus::{sender_sink, EventMux};
use dpp::version::PlatformVersion;
use std::fmt::Debug;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

const PLATFORM_EVENTS_STREAM_BUFFER: usize = 128;

/// Service to handle platform queries
pub struct QueryService {
    platform: Arc<Platform<DefaultCoreRPC>>,
    event_bus: EventBus<PlatformEvent, PlatformFilterAdapter>,
    /// Multiplexer for Platform events
    platform_events_mux: EventMux,
    /// background worker tasks
    workers: Arc<Mutex<tokio::task::JoinSet<()>>>,
}

type QueryMethod<RQ, RS> = fn(
    &Platform<DefaultCoreRPC>,
    RQ,
    &PlatformState,
    &PlatformVersion,
) -> Result<QueryValidationResult<RS>, Error>;

impl QueryService {
    /// Creates new QueryService
    pub fn new(
        platform: Arc<Platform<DefaultCoreRPC>>,
        event_bus: EventBus<PlatformEvent, PlatformFilterAdapter>,
    ) -> Self {
        let mux = EventMux::new();
        let mut workers = tokio::task::JoinSet::new();

        // Start local mux producer to bridge internal event_bus
        {
            let bus = event_bus.clone();
            let worker_mux = mux.clone();
            workers.spawn(async move {
                use std::sync::Arc;
                let mk = Arc::new(|f| PlatformFilterAdapter::new(f));
                dash_event_bus::run_local_platform_events_producer(worker_mux, bus, mk).await;
            });
        }

        Self {
            platform,
            event_bus,
            platform_events_mux: mux,
            workers: Arc::new(Mutex::new(workers)),
        }
    }

    async fn handle_blocking_query<RQ, RS>(
        &self,
        request: Request<RQ>,
        query_method: QueryMethod<RQ, RS>,
        endpoint_name: &str,
    ) -> Result<Response<RS>, Status>
    where
        RS: Clone + Send + 'static,
        RQ: Debug + Send + Clone + 'static,
    {
        let mut response_duration_metric = query_duration_metric(endpoint_name);

        let platform = Arc::clone(&self.platform);

        let request_debug = format!("{:?}", &request);

        let result = spawn_blocking_task_with_name_if_supported("query", move || {
            let mut result;

            let query_request = request.into_inner();

            let mut query_counter = 0;

            loop {
                let platform_state = platform.state.load();

                let platform_version = platform_state
                    .current_platform_version()
                    .map_err(|_| Status::unavailable("platform is not initialized"))?;

                // Query is using Platform execution state and Drive state to during the execution.
                // They are updating every block in finalize block ABCI handler.
                // The problem is that these two operations aren't atomic and some latency between
                // them could lead to data races. `committed_block_height_guard` counter that represents
                // the latest the height of latest committed Drive state and logic bellow ensures
                // that query is executed only after/before both states are updated.
                let mut needs_restart = false;

                loop {
                    let committed_block_height_guard = platform
                        .committed_block_height_guard
                        .load(Ordering::Relaxed);
                    let mut counter = 0;
                    if platform_state.last_committed_block_height() == committed_block_height_guard
                    {
                        break;
                    } else {
                        counter += 1;
                        sleep(Duration::from_millis(10))
                    }

                    // We try for up to 1 second
                    if counter >= 100 {
                        query_counter += 1;
                        needs_restart = true;
                        break;
                    }
                }

                if query_counter > 3 {
                    return Err(query_error_into_status(QueryError::NotServiceable(
                        "platform is saturated (did not attempt query)".to_string(),
                    )));
                }

                if needs_restart {
                    continue;
                }

                result = query_method(
                    &platform,
                    query_request.clone(),
                    &platform_state,
                    platform_version,
                );

                let committed_block_height_guard = platform
                    .committed_block_height_guard
                    .load(Ordering::Relaxed);

                if platform_state.last_committed_block_height() == committed_block_height_guard {
                    // in this case the query almost certainly executed correctly
                    break;
                } else {
                    query_counter += 1;

                    if query_counter > 2 {
                        // This should never be possible
                        return Err(query_error_into_status(QueryError::NotServiceable(
                            "platform is saturated".to_string(),
                        )));
                    }
                }
            }

            let mut query_result = result.map_err(error_into_status)?;

            if query_result.is_valid() {
                let response = query_result
                    .into_data()
                    .map_err(|error| error_into_status(error.into()))?;

                Ok(Response::new(response))
            } else {
                let error = query_result.errors.swap_remove(0);

                Err(query_error_into_status(error))
            }
        })?
        .instrument(tracing::trace_span!("query", endpoint_name))
        .await
        .map_err(|error| Status::internal(format!("query thread failed: {}", error)))?;

        // Query logging and metrics
        let code = match &result {
            Ok(_) => Code::Ok,
            Err(status) => status.code(),
        };

        let code_label = format!("{:?}", code).to_lowercase();

        // Add code to response duration metric
        let label = abci_response_code_metric_label(code);
        response_duration_metric.add_label(label);

        match code {
            // User errors
            Code::Ok
            | Code::InvalidArgument
            | Code::NotFound
            | Code::AlreadyExists
            | Code::ResourceExhausted
            | Code::PermissionDenied
            | Code::Unavailable
            | Code::Aborted
            | Code::FailedPrecondition
            | Code::OutOfRange
            | Code::Cancelled
            | Code::DeadlineExceeded
            | Code::Unauthenticated => {
                let elapsed_time = response_duration_metric.elapsed().as_secs_f64();

                tracing::trace!(
                    request = request_debug,
                    elapsed_time,
                    endpoint_name,
                    code = code_label,
                    "query '{}' executed with code {:?} in {} secs",
                    endpoint_name,
                    code,
                    elapsed_time
                );
            }
            // System errors
            Code::Unknown | Code::Unimplemented | Code::Internal | Code::DataLoss => {
                tracing::error!(
                    request = request_debug,
                    endpoint_name,
                    code = code_label,
                    "query '{}' execution failed with code {:?}",
                    endpoint_name,
                    code
                );
            }
        }

        result
    }
}

fn respond_with_unimplemented<RS>(name: &str) -> Result<Response<RS>, Status> {
    tracing::error!("{} endpoint is called but it's not supported", name);

    Err(Status::unimplemented("the endpoint is not supported"))
}

/// Adapter implementing EventBus filter semantics based on incoming gRPC `PlatformFilterV0`.
#[derive(Clone, Debug)]
pub struct PlatformFilterAdapter {
    inner: dapi_grpc::platform::v0::PlatformFilterV0,
}

impl PlatformFilterAdapter {
    /// Create a new adapter wrapping the provided gRPC `PlatformFilterV0`.
    pub fn new(inner: dapi_grpc::platform::v0::PlatformFilterV0) -> Self {
        Self { inner }
    }
}

impl EventBusFilter<PlatformEvent> for PlatformFilterAdapter {
    fn matches(&self, event: &PlatformEvent) -> bool {
        use dapi_grpc::platform::v0::platform_event_v0::Event as Evt;
        use dapi_grpc::platform::v0::platform_filter_v0::Kind;
        match self.inner.kind.as_ref() {
            None => false,
            Some(Kind::All(all)) => *all,
            Some(Kind::BlockCommitted(b)) => {
                if !*b {
                    return false;
                }
                matches!(event.event, Some(Evt::BlockCommitted(_)))
            }
            Some(Kind::StateTransitionResult(filter)) => {
                // If tx_hash is provided, match only that hash; otherwise match any STR
                if let Some(Evt::StateTransitionFinalized(ref r)) = event.event {
                    match &filter.tx_hash {
                        Some(h) => r.tx_hash == *h,
                        None => true,
                    }
                } else {
                    false
                }
            }
        }
    }
}

#[async_trait]
impl PlatformService for QueryService {
    async fn broadcast_state_transition(
        &self,
        _request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        respond_with_unimplemented("broadcast_state_transition")
    }

    async fn get_identity(
        &self,
        request: Request<GetIdentityRequest>,
    ) -> Result<Response<GetIdentityResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity,
            "get_identity",
        )
        .await
    }

    async fn get_identities_contract_keys(
        &self,
        request: Request<GetIdentitiesContractKeysRequest>,
    ) -> Result<Response<GetIdentitiesContractKeysResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identities_contract_keys,
            "get_identities_contract_keys",
        )
        .await
    }

    async fn get_identity_keys(
        &self,
        request: Request<GetIdentityKeysRequest>,
    ) -> Result<Response<GetIdentityKeysResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_keys,
            "get_identity_keys",
        )
        .await
    }

    async fn get_identity_nonce(
        &self,
        request: Request<GetIdentityNonceRequest>,
    ) -> Result<Response<GetIdentityNonceResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_nonce,
            "get_identity_nonce",
        )
        .await
    }

    async fn get_identity_contract_nonce(
        &self,
        request: Request<GetIdentityContractNonceRequest>,
    ) -> Result<Response<GetIdentityContractNonceResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_contract_nonce,
            "get_identity_contract_nonce",
        )
        .await
    }

    async fn get_identity_balance(
        &self,
        request: Request<GetIdentityBalanceRequest>,
    ) -> Result<Response<GetIdentityBalanceResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_balance,
            "get_identity_balance",
        )
        .await
    }

    async fn get_identity_balance_and_revision(
        &self,
        request: Request<GetIdentityBalanceAndRevisionRequest>,
    ) -> Result<Response<GetIdentityBalanceAndRevisionResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_balance_and_revision,
            "get_identity_balance_and_revision",
        )
        .await
    }

    async fn get_data_contract(
        &self,
        request: Request<GetDataContractRequest>,
    ) -> Result<Response<GetDataContractResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_data_contract,
            "get_data_contract",
        )
        .await
    }

    async fn get_data_contract_history(
        &self,
        request: Request<GetDataContractHistoryRequest>,
    ) -> Result<Response<GetDataContractHistoryResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_data_contract_history,
            "get_data_contract_history",
        )
        .await
    }

    async fn get_data_contracts(
        &self,
        request: Request<GetDataContractsRequest>,
    ) -> Result<Response<GetDataContractsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_data_contracts,
            "get_data_contracts",
        )
        .await
    }

    async fn get_documents(
        &self,
        request: Request<GetDocumentsRequest>,
    ) -> Result<Response<GetDocumentsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_documents,
            "get_documents",
        )
        .await
    }

    async fn get_identity_by_public_key_hash(
        &self,
        request: Request<GetIdentityByPublicKeyHashRequest>,
    ) -> Result<Response<GetIdentityByPublicKeyHashResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_by_public_key_hash,
            "get_identity_by_public_key_hash",
        )
        .await
    }

    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: Request<GetIdentityByNonUniquePublicKeyHashRequest>,
    ) -> Result<Response<GetIdentityByNonUniquePublicKeyHashResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_by_non_unique_public_key_hash,
            "get_identity_by_non_unique_public_key_hash",
        )
        .await
    }

    async fn wait_for_state_transition_result(
        &self,
        _request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        respond_with_unimplemented("wait_for_state_transition_result")
    }

    async fn get_consensus_params(
        &self,
        _request: Request<GetConsensusParamsRequest>,
    ) -> Result<Response<GetConsensusParamsResponse>, Status> {
        respond_with_unimplemented("get_consensus_params")
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        request: Request<GetProtocolVersionUpgradeStateRequest>,
    ) -> Result<Response<GetProtocolVersionUpgradeStateResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_version_upgrade_state,
            "get_protocol_version_upgrade_state",
        )
        .await
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: Request<GetProtocolVersionUpgradeVoteStatusRequest>,
    ) -> Result<Response<GetProtocolVersionUpgradeVoteStatusResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_version_upgrade_vote_status,
            "get_protocol_version_upgrade_vote_status",
        )
        .await
    }

    async fn get_epochs_info(
        &self,
        request: Request<GetEpochsInfoRequest>,
    ) -> Result<Response<GetEpochsInfoResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_epoch_infos,
            "get_epochs_info",
        )
        .await
    }

    async fn get_path_elements(
        &self,
        request: Request<GetPathElementsRequest>,
    ) -> Result<Response<GetPathElementsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_path_elements,
            "get_path_elements",
        )
        .await
    }

    async fn get_contested_resources(
        &self,
        request: Request<GetContestedResourcesRequest>,
    ) -> Result<Response<GetContestedResourcesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_contested_resources,
            "get_contested_resources",
        )
        .await
    }

    async fn get_contested_resource_vote_state(
        &self,
        request: Request<GetContestedResourceVoteStateRequest>,
    ) -> Result<Response<GetContestedResourceVoteStateResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_contested_resource_vote_state,
            "get_contested_resource_vote_state",
        )
        .await
    }

    async fn get_contested_resource_voters_for_identity(
        &self,
        request: Request<GetContestedResourceVotersForIdentityRequest>,
    ) -> Result<Response<GetContestedResourceVotersForIdentityResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_contested_resource_voters_for_identity,
            "get_contested_resource_voters_for_identity",
        )
        .await
    }

    async fn get_contested_resource_identity_votes(
        &self,
        request: Request<GetContestedResourceIdentityVotesRequest>,
    ) -> Result<Response<GetContestedResourceIdentityVotesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_contested_resource_identity_votes,
            "get_contested_resource_identity_votes",
        )
        .await
    }

    async fn get_vote_polls_by_end_date(
        &self,
        request: Request<GetVotePollsByEndDateRequest>,
    ) -> Result<Response<GetVotePollsByEndDateResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_vote_polls_by_end_date_query,
            "get_vote_polls_by_end_date",
        )
        .await
    }

    async fn get_prefunded_specialized_balance(
        &self,
        request: Request<GetPrefundedSpecializedBalanceRequest>,
    ) -> Result<Response<GetPrefundedSpecializedBalanceResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_prefunded_specialized_balance,
            "get_prefunded_specialized_balance",
        )
        .await
    }

    async fn get_total_credits_in_platform(
        &self,
        request: Request<GetTotalCreditsInPlatformRequest>,
    ) -> Result<Response<GetTotalCreditsInPlatformResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_total_credits_in_platform,
            "get_total_credits_in_platform",
        )
        .await
    }

    async fn get_identities_balances(
        &self,
        request: Request<GetIdentitiesBalancesRequest>,
    ) -> Result<Response<GetIdentitiesBalancesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identities_balances,
            "get_identities_balances",
        )
        .await
    }

    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_partial_status,
            "query_partial_status",
        )
        .await
    }

    async fn get_evonodes_proposed_epoch_blocks_by_ids(
        &self,
        request: Request<GetEvonodesProposedEpochBlocksByIdsRequest>,
    ) -> Result<Response<GetEvonodesProposedEpochBlocksResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_proposed_block_counts_by_evonode_ids,
            "query_proposed_block_counts_by_evonode_ids",
        )
        .await
    }

    async fn get_evonodes_proposed_epoch_blocks_by_range(
        &self,
        request: Request<GetEvonodesProposedEpochBlocksByRangeRequest>,
    ) -> Result<Response<GetEvonodesProposedEpochBlocksResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_proposed_block_counts_by_range,
            "query_proposed_block_counts_by_range",
        )
        .await
    }

    async fn get_current_quorums_info(
        &self,
        request: Request<GetCurrentQuorumsInfoRequest>,
    ) -> Result<Response<GetCurrentQuorumsInfoResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_current_quorums_info,
            "query_current_quorums_info",
        )
        .await
    }

    async fn get_identity_token_balances(
        &self,
        request: Request<GetIdentityTokenBalancesRequest>,
    ) -> Result<Response<GetIdentityTokenBalancesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_token_balances,
            "query_identity_token_balances",
        )
        .await
    }

    async fn get_identities_token_balances(
        &self,
        request: Request<GetIdentitiesTokenBalancesRequest>,
    ) -> Result<Response<GetIdentitiesTokenBalancesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identities_token_balances,
            "query_identities_token_balances",
        )
        .await
    }

    async fn get_identity_token_infos(
        &self,
        request: Request<GetIdentityTokenInfosRequest>,
    ) -> Result<Response<GetIdentityTokenInfosResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_token_infos,
            "query_identity_token_infos",
        )
        .await
    }

    async fn get_identities_token_infos(
        &self,
        request: Request<GetIdentitiesTokenInfosRequest>,
    ) -> Result<Response<GetIdentitiesTokenInfosResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_identities_token_infos,
            "query_identities_token_infos",
        )
        .await
    }

    async fn get_token_statuses(
        &self,
        request: Request<GetTokenStatusesRequest>,
    ) -> Result<Response<GetTokenStatusesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_statuses,
            "get_token_statuses",
        )
        .await
    }

    async fn get_token_pre_programmed_distributions(
        &self,
        request: Request<GetTokenPreProgrammedDistributionsRequest>,
    ) -> Result<Response<GetTokenPreProgrammedDistributionsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_pre_programmed_distributions,
            "get_token_pre_programmed_distributions",
        )
        .await
    }

    async fn get_token_total_supply(
        &self,
        request: Request<GetTokenTotalSupplyRequest>,
    ) -> Result<Response<GetTokenTotalSupplyResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_total_supply,
            "get_token_total_supply",
        )
        .await
    }

    async fn get_group_info(
        &self,
        request: Request<GetGroupInfoRequest>,
    ) -> Result<Response<GetGroupInfoResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_group_info,
            "get_group_info",
        )
        .await
    }

    async fn get_group_infos(
        &self,
        request: Request<GetGroupInfosRequest>,
    ) -> Result<Response<GetGroupInfosResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_group_infos,
            "get_group_infos",
        )
        .await
    }

    async fn get_group_actions(
        &self,
        request: Request<GetGroupActionsRequest>,
    ) -> Result<Response<GetGroupActionsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_group_actions,
            "get_group_actions",
        )
        .await
    }

    async fn get_group_action_signers(
        &self,
        request: Request<GetGroupActionSignersRequest>,
    ) -> Result<Response<GetGroupActionSignersResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_group_action_signers,
            "get_group_action_signers",
        )
        .await
    }

    async fn get_token_direct_purchase_prices(
        &self,
        request: Request<GetTokenDirectPurchasePricesRequest>,
    ) -> Result<Response<GetTokenDirectPurchasePricesResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_direct_purchase_prices,
            "get_token_direct_purchase_prices",
        )
        .await
    }

    async fn get_token_contract_info(
        &self,
        request: Request<GetTokenContractInfoRequest>,
    ) -> Result<Response<GetTokenContractInfoResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_contract_info,
            "get_token_contract_info",
        )
        .await
    }

    async fn get_token_perpetual_distribution_last_claim(
        &self,
        request: Request<GetTokenPerpetualDistributionLastClaimRequest>,
    ) -> Result<Response<GetTokenPerpetualDistributionLastClaimResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_token_perpetual_distribution_last_claim,
            "get_token_perpetual_distribution_last_claim",
        )
        .await
    }

    async fn get_finalized_epoch_infos(
        &self,
        request: Request<GetFinalizedEpochInfosRequest>,
    ) -> Result<Response<GetFinalizedEpochInfosResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_finalized_epoch_infos,
            "get_finalized_epoch_infos",
        )
        .await
    }

    type subscribePlatformEventsStream = ReceiverStream<Result<PlatformEventsResponse, Status>>;

    /// Uses EventMux: forward inbound commands to mux subscriber and return its response stream
    async fn subscribe_platform_events(
        &self,
        request: Request<Streaming<PlatformEventsCommand>>,
    ) -> Result<Response<Self::subscribePlatformEventsStream>, Status> {
        // TODO: two issues are to be resolved:
        // 1) restart of client with the same subscription id shows that old subscription is not removed
        // 2) connection drops after some time
        // return Err(Status::unimplemented("the endpoint is not supported yet"));
        let inbound = request.into_inner();
        let (downstream_tx, rx) =
            mpsc::channel::<Result<PlatformEventsResponse, Status>>(PLATFORM_EVENTS_STREAM_BUFFER);
        let subscriber = self.platform_events_mux.add_subscriber().await;

        let mut workers = self.workers.lock().unwrap();
        workers.spawn(async move {
            let resp_sink = sender_sink(downstream_tx);
            subscriber.forward(inbound, resp_sink).await;
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}

// Local event forwarding handled in dash_event_bus shared local_bus_producer

/// Local producer: consumes commands from mux and produces responses by
/// subscribing to internal `event_bus` and forwarding events as responses.
async fn run_local_platform_events_producer(
    mux: EventMux,
    event_bus: EventBus<PlatformEvent, PlatformFilterAdapter>,
) {
    use dapi_grpc::platform::v0::platform_events_command::platform_events_command_v0::Command as Cmd;
    use dapi_grpc::platform::v0::platform_events_command::Version as CmdVersion;
    use dapi_grpc::platform::v0::platform_events_response::platform_events_response_v0::Response as Resp;
    use dapi_grpc::platform::v0::platform_events_response::Version as RespVersion;

    let producer = mux.add_producer().await;
    let mut cmd_rx = producer.cmd_rx;
    let resp_tx = producer.resp_tx;

    // Connection-local subscriptions routing map
    use std::collections::HashMap;
    let mut subs: HashMap<String, SubscriptionHandle<PlatformEvent, PlatformFilterAdapter>> =
        HashMap::new();

    while let Some(cmd_res) = cmd_rx.recv().await {
        match cmd_res {
            Ok(cmd) => {
                let v0 = match cmd.version {
                    Some(CmdVersion::V0(v0)) => v0,
                    None => {
                        let err = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Error(
                                    dapi_grpc::platform::v0::PlatformErrorV0 {
                                        client_subscription_id: "".to_string(),
                                        code: 400,
                                        message: "missing version".to_string(),
                                    },
                                )),
                            })),
                        };
                        let _ = resp_tx.send(Ok(err));
                        continue;
                    }
                };
                match v0.command {
                    Some(Cmd::Add(add)) => {
                        let id = add.client_subscription_id;
                        let adapter = PlatformFilterAdapter::new(add.filter.unwrap_or_default());
                        let handle = event_bus.add_subscription(adapter).await;

                        // Start forwarding events for this subscription
                        let id_for = id.clone();
                        let handle_clone = handle.clone();
                        let resp_tx_clone = resp_tx.clone();
                        tokio::spawn(async move {
                            // forwarding handled in rs-dash-event-bus shared producer in new setup
                            let _ = (handle_clone, id_for, resp_tx_clone);
                        });

                        subs.insert(id.clone(), handle);

                        // Ack
                        let ack = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0 {
                                    client_subscription_id: id,
                                    op: "add".to_string(),
                                })),
                            })),
                        };
                        let _ = resp_tx.send(Ok(ack));
                    }
                    Some(Cmd::Remove(rem)) => {
                        let id = rem.client_subscription_id;
                        if subs.remove(&id).is_some() {
                            let ack = PlatformEventsResponse {
                                version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                    response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0 {
                                        client_subscription_id: id,
                                        op: "remove".to_string(),
                                    })),
                                })),
                            };
                            let _ = resp_tx.send(Ok(ack));
                        }
                    }
                    Some(Cmd::Ping(p)) => {
                        let ack = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Ack(dapi_grpc::platform::v0::AckV0 {
                                    client_subscription_id: p.nonce.to_string(),
                                    op: "ping".to_string(),
                                })),
                            })),
                        };
                        let _ = resp_tx.send(Ok(ack));
                    }
                    None => {
                        let err = PlatformEventsResponse {
                            version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                                response: Some(Resp::Error(
                                    dapi_grpc::platform::v0::PlatformErrorV0 {
                                        client_subscription_id: "".to_string(),
                                        code: 400,
                                        message: "missing command".to_string(),
                                    },
                                )),
                            })),
                        };
                        let _ = resp_tx.send(Ok(err));
                    }
                }
            }
            Err(e) => {
                tracing::warn!("producer received error command: {}", e);
                let err = PlatformEventsResponse {
                    version: Some(RespVersion::V0(PlatformEventsResponseV0 {
                        response: Some(Resp::Error(dapi_grpc::platform::v0::PlatformErrorV0 {
                            client_subscription_id: "".to_string(),
                            code: 500,
                            message: format!("{}", e),
                        })),
                    })),
                };
                let _ = resp_tx.send(Ok(err));
            }
        }
    }
}

#[async_trait]
impl DriveInternal for QueryService {
    async fn get_proofs(
        &self,
        request: Request<GetProofsRequest>,
    ) -> Result<Response<GetProofsResponse>, Status> {
        self.handle_blocking_query(
            request,
            Platform::<DefaultCoreRPC>::query_proofs,
            "get_proofs",
        )
        .await
    }
}

fn query_error_into_status(error: QueryError) -> Status {
    match error {
        QueryError::NotFound(message) => Status::not_found(message),
        QueryError::InvalidArgument(message) => Status::invalid_argument(message),
        QueryError::Query(error) => Status::invalid_argument(error.to_string()),
        _ => {
            tracing::error!("unexpected query error: {:?}", error);

            Status::unknown(error.to_string())
        }
    }
}

fn error_into_status(error: Error) -> Status {
    Status::internal(format!("query: {}", error))
}
