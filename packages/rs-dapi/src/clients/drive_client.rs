use crate::{DAPIResult, DapiError};
use async_trait::async_trait;
use dapi_grpc::platform::v0::{
    platform_client::PlatformClient, BroadcastStateTransitionRequest,
    BroadcastStateTransitionResponse, GetConsensusParamsRequest, GetConsensusParamsResponse,
    GetContestedResourceIdentityVotesRequest, GetContestedResourceIdentityVotesResponse,
    GetContestedResourceVoteStateRequest, GetContestedResourceVoteStateResponse,
    GetContestedResourceVotersForIdentityRequest, GetContestedResourceVotersForIdentityResponse,
    GetContestedResourcesRequest, GetContestedResourcesResponse, GetCurrentQuorumsInfoRequest,
    GetCurrentQuorumsInfoResponse, GetDataContractHistoryRequest, GetDataContractHistoryResponse,
    GetDataContractRequest, GetDataContractResponse, GetDataContractsRequest,
    GetDataContractsResponse, GetDocumentsRequest, GetDocumentsResponse, GetEpochsInfoRequest,
    GetEpochsInfoResponse, GetFinalizedEpochInfosRequest, GetFinalizedEpochInfosResponse,
    GetGroupActionSignersRequest, GetGroupActionSignersResponse, GetGroupActionsRequest,
    GetGroupActionsResponse, GetGroupInfoRequest, GetGroupInfoResponse, GetGroupInfosRequest,
    GetGroupInfosResponse, GetIdentitiesBalancesRequest, GetIdentitiesBalancesResponse,
    GetIdentitiesContractKeysRequest, GetIdentitiesContractKeysResponse,
    GetIdentitiesTokenBalancesRequest, GetIdentitiesTokenBalancesResponse,
    GetIdentitiesTokenInfosRequest, GetIdentitiesTokenInfosResponse,
    GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceAndRevisionResponse,
    GetIdentityBalanceRequest, GetIdentityBalanceResponse,
    GetIdentityByNonUniquePublicKeyHashRequest, GetIdentityByNonUniquePublicKeyHashResponse,
    GetIdentityByPublicKeyHashRequest, GetIdentityByPublicKeyHashResponse,
    GetIdentityContractNonceRequest, GetIdentityContractNonceResponse, GetIdentityKeysRequest,
    GetIdentityKeysResponse, GetIdentityNonceRequest, GetIdentityNonceResponse, GetIdentityRequest,
    GetIdentityResponse, GetIdentityTokenBalancesRequest, GetIdentityTokenBalancesResponse,
    GetIdentityTokenInfosRequest, GetIdentityTokenInfosResponse, GetPathElementsRequest,
    GetPathElementsResponse, GetPrefundedSpecializedBalanceRequest,
    GetPrefundedSpecializedBalanceResponse, GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeStateResponse, GetProtocolVersionUpgradeVoteStatusRequest,
    GetProtocolVersionUpgradeVoteStatusResponse, GetStatusRequest, GetTokenContractInfoRequest,
    GetTokenContractInfoResponse, GetTokenDirectPurchasePricesRequest,
    GetTokenDirectPurchasePricesResponse, GetTokenPerpetualDistributionLastClaimRequest,
    GetTokenPerpetualDistributionLastClaimResponse, GetTokenPreProgrammedDistributionsRequest,
    GetTokenPreProgrammedDistributionsResponse, GetTokenStatusesRequest, GetTokenStatusesResponse,
    GetTokenTotalSupplyRequest, GetTokenTotalSupplyResponse, GetTotalCreditsInPlatformRequest,
    GetTotalCreditsInPlatformResponse, GetVotePollsByEndDateRequest, GetVotePollsByEndDateResponse,
    WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use serde::{Deserialize, Serialize};

use tower::ServiceBuilder;
use tower_http::{
    trace::{
        DefaultMakeSpan, DefaultOnBodyChunk, DefaultOnEos, DefaultOnFailure, DefaultOnRequest,
        DefaultOnResponse, Trace, TraceLayer,
    },
    LatencyUnit,
};
use tracing::{error, info, trace, Level};

use super::traits::DriveClientTrait;

/// gRPC client for interacting with Dash Platform Drive
///
/// This client includes automatic gRPC request/response tracing via tonic interceptors.
/// All gRPC requests will be logged at TRACE level with:
/// - Request method and URI
/// - Response timing and status
/// - Error details for failed requests
///
/// Error handling follows client-layer architecture:
/// - Technical failures (connection errors, timeouts) are logged with `tracing::error!`
/// - Service errors (gRPC status codes like NotFound) are logged with `tracing::debug!`
///
/// The client maintains a persistent connection that is reused across requests to improve performance.
pub struct DriveClient {
    base_url: String,
    channel: DriveChannel,
    client: PlatformClient<DriveChannel>,
}

impl std::fmt::Debug for DriveClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DriveClient")
            .field("base_url", &self.base_url)
            .field("channel", &"<Channel>")
            .finish()
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveStatusResponse {
    pub version: Option<DriveVersion>,
    pub chain: Option<DriveChain>,
    pub time: Option<DriveTime>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveVersion {
    pub software: Option<DriveSoftware>,
    pub protocol: Option<DriveProtocol>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveSoftware {
    pub drive: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveProtocol {
    pub drive: Option<DriveProtocolVersion>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveProtocolVersion {
    pub current: Option<u64>,
    pub latest: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveChain {
    #[serde(rename = "coreChainLockedHeight")]
    pub core_chain_locked_height: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DriveTime {
    pub block: Option<u64>,
    pub genesis: Option<u64>,
    pub epoch: Option<u64>,
}

type DriveChannel = Trace<
    tonic::transport::Channel,
    tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
    DefaultMakeSpan,
    DefaultOnRequest,
    DefaultOnResponse,
    DefaultOnBodyChunk,
>;

impl DriveClient {
    /// Create a new DriveClient with gRPC request tracing and connection reuse
    pub async fn new(uri: &str) -> DAPIResult<Self> {
        info!("Creating Drive client for: {}", uri);
        let channel = Self::create_channel(uri).await?;

        Ok(Self {
            base_url: uri.to_string(),
            client: PlatformClient::new(channel.clone()),
            channel,
        })
    }

    async fn create_channel(uri: &str) -> DAPIResult<DriveChannel> {
        let raw_channel = dapi_grpc::tonic::transport::Endpoint::from_shared(uri.to_string())
            .map_err(|e| {
                error!("Invalid Drive service URI {}: {}", uri, e);
                DapiError::Client(format!("Invalid URI: {}", e))
            })?
            .connect()
            .await
            .map_err(|e| {
                error!("Failed to connect to Drive service at {}: {}", uri, e);
                DapiError::Client(format!("Failed to connect to Drive service: {}", e))
            })?;

        let channel: Trace<
            tonic::transport::Channel,
            tower_http::classify::SharedClassifier<tower_http::classify::ServerErrorsAsFailures>,
            DefaultMakeSpan,
            DefaultOnRequest,
            DefaultOnResponse,
            DefaultOnBodyChunk,
        > = ServiceBuilder::new()
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(DefaultMakeSpan::new().include_headers(true))
                    .on_request(DefaultOnRequest::new().level(Level::TRACE))
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Micros),
                    )
                    .on_failure(DefaultOnFailure::new().level(Level::WARN))
                    .on_eos(DefaultOnEos::new().level(Level::DEBUG))
                    .on_body_chunk(DefaultOnBodyChunk::new()),
            )
            .service(raw_channel);

        Ok(channel)
    }

    pub async fn get_status(&self, request: &GetStatusRequest) -> DAPIResult<DriveStatusResponse> {
        let start_time = std::time::Instant::now();

        // Get client with tracing interceptor (reuses cached connection)
        let mut client = self.get_client().await?;

        trace!("Making get_status gRPC call to Drive");
        // Make gRPC call to Drive with timing
        let response = client
            .get_status(dapi_grpc::tonic::Request::new(*request))
            .await;

        let drive_response = response?.into_inner();

        // Convert Drive's GetStatusResponse to our DriveStatusResponse format
        if let Some(dapi_grpc::platform::v0::get_status_response::Version::V0(v0)) =
            drive_response.version
        {
            let mut drive_status = DriveStatusResponse::default();

            // Extract version information
            if let Some(version) = v0.version {
                let mut drive_version = DriveVersion::default();

                if let Some(software) = version.software {
                    drive_version.software = Some(DriveSoftware {
                        drive: software.drive,
                    });
                }

                if let Some(protocol) = version.protocol {
                    if let Some(drive_proto) = protocol.drive {
                        drive_version.protocol = Some(DriveProtocol {
                            drive: Some(DriveProtocolVersion {
                                current: Some(drive_proto.current as u64),
                                latest: Some(drive_proto.latest as u64),
                            }),
                        });
                    }
                }

                drive_status.version = Some(drive_version);
            }

            // Extract chain information
            if let Some(chain) = v0.chain {
                drive_status.chain = Some(DriveChain {
                    core_chain_locked_height: chain.core_chain_locked_height.map(|h| h as u64),
                });
            }

            // Extract time information
            if let Some(time) = v0.time {
                drive_status.time = Some(DriveTime {
                    block: Some(time.local),
                    genesis: time.genesis,
                    epoch: time.epoch.map(|e| e as u64),
                });
            }

            Ok(drive_status)
        } else {
            Err(DapiError::Server(
                "Drive returned unexpected response format".to_string(),
            ))
        }
    }
}

#[async_trait]
impl DriveClientTrait for DriveClient {
    async fn get_status(&self, request: &GetStatusRequest) -> DAPIResult<DriveStatusResponse> {
        self.get_status(request).await
    }

    // Identity-related methods
    async fn get_identity(&self, request: &GetIdentityRequest) -> DAPIResult<GetIdentityResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_keys(
        &self,
        request: &GetIdentityKeysRequest,
    ) -> DAPIResult<GetIdentityKeysResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_keys(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_contract_keys(
        &self,
        request: &GetIdentitiesContractKeysRequest,
    ) -> DAPIResult<GetIdentitiesContractKeysResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_contract_keys(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_nonce(
        &self,
        request: &GetIdentityNonceRequest,
    ) -> DAPIResult<GetIdentityNonceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_nonce(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_contract_nonce(
        &self,
        request: &GetIdentityContractNonceRequest,
    ) -> DAPIResult<GetIdentityContractNonceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_contract_nonce(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_balance(
        &self,
        request: &GetIdentityBalanceRequest,
    ) -> DAPIResult<GetIdentityBalanceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_balance(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_balances(
        &self,
        request: &GetIdentitiesBalancesRequest,
    ) -> DAPIResult<GetIdentitiesBalancesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_balances(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_balance_and_revision(
        &self,
        request: &GetIdentityBalanceAndRevisionRequest,
    ) -> DAPIResult<GetIdentityBalanceAndRevisionResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_balance_and_revision(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_by_public_key_hash(
        &self,
        request: &GetIdentityByPublicKeyHashRequest,
    ) -> DAPIResult<GetIdentityByPublicKeyHashResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_by_public_key_hash(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: &GetIdentityByNonUniquePublicKeyHashRequest,
    ) -> DAPIResult<GetIdentityByNonUniquePublicKeyHashResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_by_non_unique_public_key_hash(dapi_grpc::tonic::Request::new(
                request.clone(),
            ))
            .await?;
        Ok(response.into_inner())
    }

    // Data Contract methods
    async fn get_data_contract(
        &self,
        request: &GetDataContractRequest,
    ) -> DAPIResult<GetDataContractResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contract(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_data_contracts(
        &self,
        request: &GetDataContractsRequest,
    ) -> DAPIResult<GetDataContractsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contracts(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_data_contract_history(
        &self,
        request: &GetDataContractHistoryRequest,
    ) -> DAPIResult<GetDataContractHistoryResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contract_history(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // Document methods
    async fn get_documents(
        &self,
        request: &GetDocumentsRequest,
    ) -> DAPIResult<GetDocumentsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_documents(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // Epoch and consensus methods
    async fn get_epochs_info(
        &self,
        request: &GetEpochsInfoRequest,
    ) -> DAPIResult<GetEpochsInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_epochs_info(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_finalized_epoch_infos(
        &self,
        request: &GetFinalizedEpochInfosRequest,
    ) -> DAPIResult<GetFinalizedEpochInfosResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_finalized_epoch_infos(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_consensus_params(
        &self,
        request: &GetConsensusParamsRequest,
    ) -> DAPIResult<GetConsensusParamsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_consensus_params(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        request: &GetProtocolVersionUpgradeStateRequest,
    ) -> DAPIResult<GetProtocolVersionUpgradeStateResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_protocol_version_upgrade_state(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: &GetProtocolVersionUpgradeVoteStatusRequest,
    ) -> DAPIResult<GetProtocolVersionUpgradeVoteStatusResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_protocol_version_upgrade_vote_status(dapi_grpc::tonic::Request::new(
                request.clone(),
            ))
            .await?;
        Ok(response.into_inner())
    }

    // Other methods
    async fn get_path_elements(
        &self,
        request: &GetPathElementsRequest,
    ) -> DAPIResult<GetPathElementsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_path_elements(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_total_credits_in_platform(
        &self,
        request: &GetTotalCreditsInPlatformRequest,
    ) -> DAPIResult<GetTotalCreditsInPlatformResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_total_credits_in_platform(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_current_quorums_info(
        &self,
        request: &GetCurrentQuorumsInfoRequest,
    ) -> DAPIResult<GetCurrentQuorumsInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_current_quorums_info(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    // Contested resource methods
    async fn get_contested_resources(
        &self,
        request: &GetContestedResourcesRequest,
    ) -> DAPIResult<GetContestedResourcesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_contested_resources(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_contested_resource_vote_state(
        &self,
        request: &GetContestedResourceVoteStateRequest,
    ) -> DAPIResult<GetContestedResourceVoteStateResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_contested_resource_vote_state(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_contested_resource_voters_for_identity(
        &self,
        request: &GetContestedResourceVotersForIdentityRequest,
    ) -> DAPIResult<GetContestedResourceVotersForIdentityResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_contested_resource_voters_for_identity(dapi_grpc::tonic::Request::new(
                request.clone(),
            ))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_contested_resource_identity_votes(
        &self,
        request: &GetContestedResourceIdentityVotesRequest,
    ) -> DAPIResult<GetContestedResourceIdentityVotesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_contested_resource_identity_votes(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_vote_polls_by_end_date(
        &self,
        request: &GetVotePollsByEndDateRequest,
    ) -> DAPIResult<GetVotePollsByEndDateResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_vote_polls_by_end_date(dapi_grpc::tonic::Request::new(*request))
            .await?;
        Ok(response.into_inner())
    }

    // Token methods
    async fn get_identity_token_balances(
        &self,
        request: &GetIdentityTokenBalancesRequest,
    ) -> DAPIResult<GetIdentityTokenBalancesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_token_balances(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_token_balances(
        &self,
        request: &GetIdentitiesTokenBalancesRequest,
    ) -> DAPIResult<GetIdentitiesTokenBalancesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_token_balances(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_token_infos(
        &self,
        request: &GetIdentityTokenInfosRequest,
    ) -> DAPIResult<GetIdentityTokenInfosResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_token_infos(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_token_infos(
        &self,
        request: &GetIdentitiesTokenInfosRequest,
    ) -> DAPIResult<GetIdentitiesTokenInfosResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_token_infos(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_statuses(
        &self,
        request: &GetTokenStatusesRequest,
    ) -> DAPIResult<GetTokenStatusesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_statuses(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_direct_purchase_prices(
        &self,
        request: &GetTokenDirectPurchasePricesRequest,
    ) -> DAPIResult<GetTokenDirectPurchasePricesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_direct_purchase_prices(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_contract_info(
        &self,
        request: &GetTokenContractInfoRequest,
    ) -> DAPIResult<GetTokenContractInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_contract_info(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_pre_programmed_distributions(
        &self,
        request: &GetTokenPreProgrammedDistributionsRequest,
    ) -> DAPIResult<GetTokenPreProgrammedDistributionsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_pre_programmed_distributions(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_perpetual_distribution_last_claim(
        &self,
        request: &GetTokenPerpetualDistributionLastClaimRequest,
    ) -> DAPIResult<GetTokenPerpetualDistributionLastClaimResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_perpetual_distribution_last_claim(dapi_grpc::tonic::Request::new(
                request.clone(),
            ))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_token_total_supply(
        &self,
        request: &GetTokenTotalSupplyRequest,
    ) -> DAPIResult<GetTokenTotalSupplyResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_token_total_supply(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_prefunded_specialized_balance(
        &self,
        request: &GetPrefundedSpecializedBalanceRequest,
    ) -> DAPIResult<GetPrefundedSpecializedBalanceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_prefunded_specialized_balance(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // Group methods
    async fn get_group_info(
        &self,
        request: &GetGroupInfoRequest,
    ) -> DAPIResult<GetGroupInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_group_info(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_group_infos(
        &self,
        request: &GetGroupInfosRequest,
    ) -> DAPIResult<GetGroupInfosResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_group_infos(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_group_actions(
        &self,
        request: &GetGroupActionsRequest,
    ) -> DAPIResult<GetGroupActionsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_group_actions(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_group_action_signers(
        &self,
        request: &GetGroupActionSignersRequest,
    ) -> DAPIResult<GetGroupActionSignersResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_group_action_signers(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // State transition methods
    async fn broadcast_state_transition(
        &self,
        request: &BroadcastStateTransitionRequest,
    ) -> DAPIResult<BroadcastStateTransitionResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .broadcast_state_transition(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn wait_for_state_transition_result(
        &self,
        request: &WaitForStateTransitionResultRequest,
    ) -> DAPIResult<WaitForStateTransitionResultResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .wait_for_state_transition_result(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }
}

impl DriveClient {
    /// Helper method to get a connected client with tracing interceptor
    ///
    /// This method provides a unified interface for all DriveClient trait methods,
    /// ensuring that every gRPC request benefits from:
    /// - Connection reuse (cached channel)
    /// - Automatic request/response tracing
    /// - Consistent error handling and logging
    ///
    /// All methods in the DriveClientTrait implementation use this method,
    /// providing consistent behavior across the entire client.
    async fn get_client(&self) -> DAPIResult<PlatformClient<DriveChannel>> {
        Ok(self.client.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::Request;

    #[tokio::test]
    async fn test_drive_client_tracing_integration() {
        // Test that DriveClient can be created with tracing interceptor
        let client = DriveClient::new("http://localhost:1443").await.unwrap();

        // Verify basic structure
        assert_eq!(client.base_url, "http://localhost:1443");

        // Note: In a real integration test with a running Drive instance,
        // you would see tracing logs like:
        // [TRACE] Sending gRPC request
        // [TRACE] gRPC request successful (status: OK, duration: 45ms)
        //
        // The interceptor and log_grpc_result function automatically log:
        // - Request method and timing
        // - Response status and duration
        // - Error classification (technical vs service errors)
    }
}
