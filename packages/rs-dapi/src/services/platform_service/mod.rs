// Platform service modular implementation
// This file contains the core PlatformServiceImpl struct and delegates to individual modules

mod broadcast_state_transition;
mod error_mapping;
mod get_status;
mod subscribe_platform_events;
mod wait_for_state_transition_result;

use dapi_grpc::platform::v0::platform_server::Platform;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, GetStatusRequest,
    GetStatusResponse, WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use futures::FutureExt;
use rs_dash_notify::EventMux;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tokio::time::timeout;

/// Macro to generate Platform trait method implementations that delegate to DriveClient
///
/// Usage: `drive_method!(method_name, RequestType, ResponseType);`
///
/// This generates a non-async method that returns impl Future, which:
/// 1. Gets the gRPC client from drive_client
/// 2. Calls the corresponding method on the client
/// 3. Returns the response directly (since gRPC client already returns Response<T>)
macro_rules! drive_method {
    ($method_name:ident, $request_type:ty, $response_type:ty) => {
        fn $method_name<'life0, 'async_trait>(
            &'life0 self,
            request: Request<$request_type>,
        ) -> Pin<
            Box<
                dyn Future<Output = Result<Response<$response_type>, Status>> + Send + 'async_trait,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
        {
            use crate::cache::make_cache_key;
            use crate::metrics;
            let mut client = self.drive_client.get_client();
            let cache = self.platform_cache.clone();
            let method = stringify!($method_name);
            async move {
                // Build cache key from method + request bytes
                let key = make_cache_key(method, request.get_ref());

                // Try cache
                if let Some(decoded) = cache.get(&key).await as Option<$response_type> {
                    metrics::cache_hit(method);
                    return Ok(Response::new(decoded));
                }

                // Fetch from Drive
                let resp = client.$method_name(request).await?;
                metrics::cache_miss(method);

                // Store in cache using inner message
                cache.put(key, resp.get_ref()).await;

                Ok(resp)
            }
            .boxed()
        }
    };
}

use crate::clients::tenderdash_websocket::TenderdashWebSocketClient;
use crate::config::Config;
use crate::services::streaming_service::FilterType;

/// Platform service implementation with modular method delegation
#[derive(Clone)]
pub struct PlatformServiceImpl {
    pub drive_client: crate::clients::drive_client::DriveClient,
    pub tenderdash_client: Arc<dyn crate::clients::traits::TenderdashClientTrait>,
    pub websocket_client: Arc<TenderdashWebSocketClient>,
    pub config: Arc<Config>,
    pub platform_cache: crate::cache::LruResponseCache,
    pub subscriber_manager: Arc<crate::services::streaming_service::SubscriberManager>,
    pub platform_events_mux: EventMux,
    workers: Arc<Mutex<JoinSet<()>>>,
}

impl PlatformServiceImpl {
    pub async fn new(
        drive_client: crate::clients::drive_client::DriveClient,
        tenderdash_client: Arc<dyn crate::clients::traits::TenderdashClientTrait>,
        config: Arc<Config>,
        subscriber_manager: Arc<crate::services::streaming_service::SubscriberManager>,
    ) -> Self {
        let mut workers = JoinSet::new();
        // Create WebSocket client
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(
            config.dapi.tenderdash.websocket_uri.clone(),
            1000,
        ));
        {
            let ws: Arc<TenderdashWebSocketClient> = websocket_client.clone();
            workers.spawn(async move {
                let _ = ws.connect_and_listen().await;
            });
        }

        let invalidation_subscription = subscriber_manager
            .add_subscription(FilterType::PlatformAllBlocks)
            .await;
        let event_mux = EventMux::new();

        let mux_client = drive_client.get_client().clone();
        let worker_mux = event_mux.clone();

        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        workers.spawn(async {
            if let Err(e) =
                rs_dash_notify::GrpcPlatformEventsProducer::run(worker_mux, mux_client, ready_tx)
                    .await
            {
                tracing::error!("platform events producer terminated: {}", e);
            }
        });

        if timeout(Duration::from_secs(5), ready_rx).await.is_err() {
            tracing::warn!(
                "timeout waiting for platform events producer to be ready; contonuing anyway"
            );
        }

        Self {
            drive_client,
            tenderdash_client,
            websocket_client,
            config,
            platform_cache: crate::cache::LruResponseCache::new(1024, invalidation_subscription),
            subscriber_manager,
            platform_events_mux: event_mux,
            workers: Arc::new(Mutex::new(workers)),
        }
    }
}

#[async_trait::async_trait]
impl Platform for PlatformServiceImpl {
    // Manually implemented methods

    /// Get the status of the whole system
    ///
    /// This method retrieves the current status of Drive, Tenderdash, and other components.
    ///
    /// See [`PlatformServiceImpl::get_status_impl`] for the implementation details.
    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        tracing::trace!(?request, "Received get_status request");
        self.get_status_impl(request).await
    }

    // State transition methods
    /// Broadcast a state transition to the Dash Platform
    ///
    /// This method handles the complete broadcast flow including:
    /// - State transition validation
    /// - Broadcasting to Tenderdash
    /// - Complex error handling and duplicate detection
    ///
    /// See [`PlatformServiceImpl::broadcast_state_transition_impl`] for implementation details.
    async fn broadcast_state_transition(
        &self,
        request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        tracing::trace!(?request, "Received broadcast_state_transition request");
        self.broadcast_state_transition_impl(request).await
    }

    /// Implementation of waitForStateTransitionResult
    ///
    /// This method waits for a state transition to be processed and returns the result.
    /// See [`PlatformServiceImpl::wait_for_state_transition_result_impl`] for implementation details.
    async fn wait_for_state_transition_result(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        tracing::trace!(
            ?request,
            "Received wait_for_state_transition_result request"
        );
        self.wait_for_state_transition_result_impl(request).await
    }

    // Identity-related methods
    drive_method!(
        get_identity,
        dapi_grpc::platform::v0::GetIdentityRequest,
        dapi_grpc::platform::v0::GetIdentityResponse
    );
    drive_method!(
        get_identity_keys,
        dapi_grpc::platform::v0::GetIdentityKeysRequest,
        dapi_grpc::platform::v0::GetIdentityKeysResponse
    );
    drive_method!(
        get_identities_contract_keys,
        dapi_grpc::platform::v0::GetIdentitiesContractKeysRequest,
        dapi_grpc::platform::v0::GetIdentitiesContractKeysResponse
    );
    drive_method!(
        get_identity_nonce,
        dapi_grpc::platform::v0::GetIdentityNonceRequest,
        dapi_grpc::platform::v0::GetIdentityNonceResponse
    );

    drive_method!(
        get_identity_contract_nonce,
        dapi_grpc::platform::v0::GetIdentityContractNonceRequest,
        dapi_grpc::platform::v0::GetIdentityContractNonceResponse
    );

    drive_method!(
        get_identity_balance,
        dapi_grpc::platform::v0::GetIdentityBalanceRequest,
        dapi_grpc::platform::v0::GetIdentityBalanceResponse
    );

    drive_method!(
        get_identities_balances,
        dapi_grpc::platform::v0::GetIdentitiesBalancesRequest,
        dapi_grpc::platform::v0::GetIdentitiesBalancesResponse
    );

    drive_method!(
        get_identity_balance_and_revision,
        dapi_grpc::platform::v0::GetIdentityBalanceAndRevisionRequest,
        dapi_grpc::platform::v0::GetIdentityBalanceAndRevisionResponse
    );

    drive_method!(
        get_identity_by_public_key_hash,
        dapi_grpc::platform::v0::GetIdentityByPublicKeyHashRequest,
        dapi_grpc::platform::v0::GetIdentityByPublicKeyHashResponse
    );

    drive_method!(
        get_identity_by_non_unique_public_key_hash,
        dapi_grpc::platform::v0::GetIdentityByNonUniquePublicKeyHashRequest,
        dapi_grpc::platform::v0::GetIdentityByNonUniquePublicKeyHashResponse
    );

    // Evonodes methods
    drive_method!(
        get_evonodes_proposed_epoch_blocks_by_ids,
        dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByIdsRequest,
        dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksResponse
    );

    drive_method!(
        get_evonodes_proposed_epoch_blocks_by_range,
        dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByRangeRequest,
        dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksResponse
    );

    // Data contract methods
    drive_method!(
        get_data_contract,
        dapi_grpc::platform::v0::GetDataContractRequest,
        dapi_grpc::platform::v0::GetDataContractResponse
    );

    drive_method!(
        get_data_contract_history,
        dapi_grpc::platform::v0::GetDataContractHistoryRequest,
        dapi_grpc::platform::v0::GetDataContractHistoryResponse
    );

    drive_method!(
        get_data_contracts,
        dapi_grpc::platform::v0::GetDataContractsRequest,
        dapi_grpc::platform::v0::GetDataContractsResponse
    );

    // Document methods
    drive_method!(
        get_documents,
        dapi_grpc::platform::v0::GetDocumentsRequest,
        dapi_grpc::platform::v0::GetDocumentsResponse
    );

    // System methods
    drive_method!(
        get_consensus_params,
        dapi_grpc::platform::v0::GetConsensusParamsRequest,
        dapi_grpc::platform::v0::GetConsensusParamsResponse
    );

    drive_method!(
        get_protocol_version_upgrade_state,
        dapi_grpc::platform::v0::GetProtocolVersionUpgradeStateRequest,
        dapi_grpc::platform::v0::GetProtocolVersionUpgradeStateResponse
    );

    drive_method!(
        get_protocol_version_upgrade_vote_status,
        dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusRequest,
        dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusResponse
    );

    drive_method!(
        get_epochs_info,
        dapi_grpc::platform::v0::GetEpochsInfoRequest,
        dapi_grpc::platform::v0::GetEpochsInfoResponse
    );

    drive_method!(
        get_finalized_epoch_infos,
        dapi_grpc::platform::v0::GetFinalizedEpochInfosRequest,
        dapi_grpc::platform::v0::GetFinalizedEpochInfosResponse
    );

    drive_method!(
        get_path_elements,
        dapi_grpc::platform::v0::GetPathElementsRequest,
        dapi_grpc::platform::v0::GetPathElementsResponse
    );

    drive_method!(
        get_total_credits_in_platform,
        dapi_grpc::platform::v0::GetTotalCreditsInPlatformRequest,
        dapi_grpc::platform::v0::GetTotalCreditsInPlatformResponse
    );

    // Quorum methods
    drive_method!(
        get_current_quorums_info,
        dapi_grpc::platform::v0::GetCurrentQuorumsInfoRequest,
        dapi_grpc::platform::v0::GetCurrentQuorumsInfoResponse
    );

    // Contested resource methods
    drive_method!(
        get_contested_resources,
        dapi_grpc::platform::v0::GetContestedResourcesRequest,
        dapi_grpc::platform::v0::GetContestedResourcesResponse
    );

    drive_method!(
        get_prefunded_specialized_balance,
        dapi_grpc::platform::v0::GetPrefundedSpecializedBalanceRequest,
        dapi_grpc::platform::v0::GetPrefundedSpecializedBalanceResponse
    );

    drive_method!(
        get_contested_resource_vote_state,
        dapi_grpc::platform::v0::GetContestedResourceVoteStateRequest,
        dapi_grpc::platform::v0::GetContestedResourceVoteStateResponse
    );

    drive_method!(
        get_contested_resource_voters_for_identity,
        dapi_grpc::platform::v0::GetContestedResourceVotersForIdentityRequest,
        dapi_grpc::platform::v0::GetContestedResourceVotersForIdentityResponse
    );

    drive_method!(
        get_contested_resource_identity_votes,
        dapi_grpc::platform::v0::GetContestedResourceIdentityVotesRequest,
        dapi_grpc::platform::v0::GetContestedResourceIdentityVotesResponse
    );

    drive_method!(
        get_vote_polls_by_end_date,
        dapi_grpc::platform::v0::GetVotePollsByEndDateRequest,
        dapi_grpc::platform::v0::GetVotePollsByEndDateResponse
    );

    // Token balance methods
    drive_method!(
        get_identity_token_balances,
        dapi_grpc::platform::v0::GetIdentityTokenBalancesRequest,
        dapi_grpc::platform::v0::GetIdentityTokenBalancesResponse
    );

    drive_method!(
        get_identities_token_balances,
        dapi_grpc::platform::v0::GetIdentitiesTokenBalancesRequest,
        dapi_grpc::platform::v0::GetIdentitiesTokenBalancesResponse
    );

    // Token info methods
    drive_method!(
        get_identity_token_infos,
        dapi_grpc::platform::v0::GetIdentityTokenInfosRequest,
        dapi_grpc::platform::v0::GetIdentityTokenInfosResponse
    );

    drive_method!(
        get_identities_token_infos,
        dapi_grpc::platform::v0::GetIdentitiesTokenInfosRequest,
        dapi_grpc::platform::v0::GetIdentitiesTokenInfosResponse
    );

    // Token status and pricing methods
    drive_method!(
        get_token_statuses,
        dapi_grpc::platform::v0::GetTokenStatusesRequest,
        dapi_grpc::platform::v0::GetTokenStatusesResponse
    );

    drive_method!(
        get_token_direct_purchase_prices,
        dapi_grpc::platform::v0::GetTokenDirectPurchasePricesRequest,
        dapi_grpc::platform::v0::GetTokenDirectPurchasePricesResponse
    );

    drive_method!(
        get_token_contract_info,
        dapi_grpc::platform::v0::GetTokenContractInfoRequest,
        dapi_grpc::platform::v0::GetTokenContractInfoResponse
    );

    // Token distribution methods
    drive_method!(
        get_token_pre_programmed_distributions,
        dapi_grpc::platform::v0::GetTokenPreProgrammedDistributionsRequest,
        dapi_grpc::platform::v0::GetTokenPreProgrammedDistributionsResponse
    );

    drive_method!(
        get_token_perpetual_distribution_last_claim,
        dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest,
        dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimResponse
    );

    drive_method!(
        get_token_total_supply,
        dapi_grpc::platform::v0::GetTokenTotalSupplyRequest,
        dapi_grpc::platform::v0::GetTokenTotalSupplyResponse
    );

    // Group methods
    drive_method!(
        get_group_info,
        dapi_grpc::platform::v0::GetGroupInfoRequest,
        dapi_grpc::platform::v0::GetGroupInfoResponse
    );

    drive_method!(
        get_group_infos,
        dapi_grpc::platform::v0::GetGroupInfosRequest,
        dapi_grpc::platform::v0::GetGroupInfosResponse
    );

    drive_method!(
        get_group_actions,
        dapi_grpc::platform::v0::GetGroupActionsRequest,
        dapi_grpc::platform::v0::GetGroupActionsResponse
    );

    drive_method!(
        get_group_action_signers,
        dapi_grpc::platform::v0::GetGroupActionSignersRequest,
        dapi_grpc::platform::v0::GetGroupActionSignersResponse
    );

    // Streaming: multiplexed platform events
    type subscribePlatformEventsStream = tokio_stream::wrappers::UnboundedReceiverStream<
        Result<dapi_grpc::platform::v0::PlatformEventsResponse, dapi_grpc::tonic::Status>,
    >;

    async fn subscribe_platform_events(
        &self,
        request: dapi_grpc::tonic::Request<
            dapi_grpc::tonic::Streaming<dapi_grpc::platform::v0::PlatformEventsCommand>,
        >,
    ) -> Result<
        dapi_grpc::tonic::Response<Self::subscribePlatformEventsStream>,
        dapi_grpc::tonic::Status,
    > {
        self.subscribe_platform_events_impl(request).await
    }
}
