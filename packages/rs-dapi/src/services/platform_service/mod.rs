// Platform service modular implementation
// This file contains the core PlatformServiceImpl struct and delegates to individual modules

mod broadcast_state_transition;
mod get_status;
mod wait_for_state_transition_result;

use dapi_grpc::platform::v0::platform_server::Platform;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, GetStatusRequest,
    GetStatusResponse, WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::clients::tenderdash_websocket::TenderdashWebSocketClient;
use crate::config::Config;

/// Platform service implementation with modular method delegation
#[derive(Clone)]
pub struct PlatformServiceImpl {
    pub drive_client: Arc<dyn crate::clients::traits::DriveClientTrait>,
    pub tenderdash_client: Arc<dyn crate::clients::traits::TenderdashClientTrait>,
    pub websocket_client: Arc<TenderdashWebSocketClient>,
    pub config: Arc<Config>,
}

impl PlatformServiceImpl {
    pub fn new(
        drive_client: Arc<dyn crate::clients::traits::DriveClientTrait>,
        tenderdash_client: Arc<dyn crate::clients::traits::TenderdashClientTrait>,
        config: Arc<Config>,
    ) -> Self {
        // Create WebSocket client
        let websocket_client = Arc::new(TenderdashWebSocketClient::new(
            config.dapi.tenderdash.websocket_uri.clone(),
            1000,
        ));

        Self {
            drive_client,
            tenderdash_client,
            websocket_client,
            config,
        }
    }
}

#[dapi_grpc::tonic::async_trait]
impl Platform for PlatformServiceImpl {
    async fn broadcast_state_transition(
        &self,
        request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        self.broadcast_state_transition_impl(request).await
    }

    async fn get_status(
        &self,
        request: Request<GetStatusRequest>,
    ) -> Result<Response<GetStatusResponse>, Status> {
        self.get_status_impl(request).await
    }

    async fn wait_for_state_transition_result(
        &self,
        request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        self.wait_for_state_transition_result_impl(request).await
    }

    // Identity-related methods
    async fn get_identity(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityResponse>, Status> {
        match self.drive_client.get_identity(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_keys(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityKeysRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityKeysResponse>, Status> {
        match self.drive_client.get_identity_keys(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identities_contract_keys(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentitiesContractKeysRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentitiesContractKeysResponse>, Status> {
        match self
            .drive_client
            .get_identities_contract_keys(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_nonce(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityNonceRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityNonceResponse>, Status> {
        match self
            .drive_client
            .get_identity_nonce(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_contract_nonce(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityContractNonceRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityContractNonceResponse>, Status> {
        match self
            .drive_client
            .get_identity_contract_nonce(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_balance(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityBalanceRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityBalanceResponse>, Status> {
        match self
            .drive_client
            .get_identity_balance(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identities_balances(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentitiesBalancesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentitiesBalancesResponse>, Status> {
        match self
            .drive_client
            .get_identities_balances(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_balance_and_revision(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityBalanceAndRevisionRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityBalanceAndRevisionResponse>, Status>
    {
        match self
            .drive_client
            .get_identity_balance_and_revision(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_by_public_key_hash(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityByPublicKeyHashRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityByPublicKeyHashResponse>, Status> {
        match self
            .drive_client
            .get_identity_by_public_key_hash(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: Request<dapi_grpc::platform::v0::GetIdentityByNonUniquePublicKeyHashRequest>,
    ) -> Result<
        Response<dapi_grpc::platform::v0::GetIdentityByNonUniquePublicKeyHashResponse>,
        Status,
    > {
        match self
            .drive_client
            .get_identity_by_non_unique_public_key_hash(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    // Evonodes methods (not implemented)
    async fn get_evonodes_proposed_epoch_blocks_by_ids(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByIdsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksResponse>, Status>
    {
        Err(Status::unimplemented("not implemented"))
    }

    async fn get_evonodes_proposed_epoch_blocks_by_range(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksByRangeRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetEvonodesProposedEpochBlocksResponse>, Status>
    {
        Err(Status::unimplemented("not implemented"))
    }

    // Data contract methods
    async fn get_data_contract(
        &self,
        request: Request<dapi_grpc::platform::v0::GetDataContractRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetDataContractResponse>, Status> {
        match self.drive_client.get_data_contract(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_data_contract_history(
        &self,
        request: Request<dapi_grpc::platform::v0::GetDataContractHistoryRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetDataContractHistoryResponse>, Status> {
        match self
            .drive_client
            .get_data_contract_history(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_data_contracts(
        &self,
        request: Request<dapi_grpc::platform::v0::GetDataContractsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetDataContractsResponse>, Status> {
        match self
            .drive_client
            .get_data_contracts(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    // Document methods
    async fn get_documents(
        &self,
        request: Request<dapi_grpc::platform::v0::GetDocumentsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetDocumentsResponse>, Status> {
        match self.drive_client.get_documents(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    // Consensus and protocol methods
    async fn get_consensus_params(
        &self,
        request: Request<dapi_grpc::platform::v0::GetConsensusParamsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetConsensusParamsResponse>, Status> {
        match self
            .drive_client
            .get_consensus_params(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        request: Request<dapi_grpc::platform::v0::GetProtocolVersionUpgradeStateRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetProtocolVersionUpgradeStateResponse>, Status>
    {
        match self
            .drive_client
            .get_protocol_version_upgrade_state(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: Request<dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusRequest>,
    ) -> Result<
        Response<dapi_grpc::platform::v0::GetProtocolVersionUpgradeVoteStatusResponse>,
        Status,
    > {
        match self
            .drive_client
            .get_protocol_version_upgrade_vote_status(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_epochs_info(
        &self,
        request: Request<dapi_grpc::platform::v0::GetEpochsInfoRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetEpochsInfoResponse>, Status> {
        match self.drive_client.get_epochs_info(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_finalized_epoch_infos(
        &self,
        request: Request<dapi_grpc::platform::v0::GetFinalizedEpochInfosRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetFinalizedEpochInfosResponse>, Status> {
        match self
            .drive_client
            .get_finalized_epoch_infos(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    // Other platform methods
    async fn get_path_elements(
        &self,
        request: Request<dapi_grpc::platform::v0::GetPathElementsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetPathElementsResponse>, Status> {
        match self.drive_client.get_path_elements(request.get_ref()).await {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_total_credits_in_platform(
        &self,
        request: Request<dapi_grpc::platform::v0::GetTotalCreditsInPlatformRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTotalCreditsInPlatformResponse>, Status> {
        match self
            .drive_client
            .get_total_credits_in_platform(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    async fn get_current_quorums_info(
        &self,
        request: Request<dapi_grpc::platform::v0::GetCurrentQuorumsInfoRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetCurrentQuorumsInfoResponse>, Status> {
        match self
            .drive_client
            .get_current_quorums_info(request.get_ref())
            .await
        {
            Ok(response) => Ok(Response::new(response)),
            Err(e) => Err(Status::internal(format!("Drive client error: {}", e))),
        }
    }

    // All other methods return unimplemented for now

    async fn get_contested_resources(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetContestedResourcesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetContestedResourcesResponse>, Status> {
        Err(Status::unimplemented(
            "get_contested_resources not implemented",
        ))
    }

    async fn get_prefunded_specialized_balance(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetPrefundedSpecializedBalanceRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetPrefundedSpecializedBalanceResponse>, Status>
    {
        Err(Status::unimplemented(
            "get_prefunded_specialized_balance not implemented",
        ))
    }

    async fn get_contested_resource_vote_state(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetContestedResourceVoteStateRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetContestedResourceVoteStateResponse>, Status>
    {
        Err(Status::unimplemented(
            "get_contested_resource_vote_state not implemented",
        ))
    }

    async fn get_contested_resource_voters_for_identity(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetContestedResourceVotersForIdentityRequest>,
    ) -> Result<
        Response<dapi_grpc::platform::v0::GetContestedResourceVotersForIdentityResponse>,
        Status,
    > {
        Err(Status::unimplemented(
            "get_contested_resource_voters_for_identity not implemented",
        ))
    }

    async fn get_contested_resource_identity_votes(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetContestedResourceIdentityVotesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetContestedResourceIdentityVotesResponse>, Status>
    {
        Err(Status::unimplemented(
            "get_contested_resource_identity_votes not implemented",
        ))
    }

    async fn get_vote_polls_by_end_date(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetVotePollsByEndDateRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetVotePollsByEndDateResponse>, Status> {
        Err(Status::unimplemented(
            "get_vote_polls_by_end_date not implemented",
        ))
    }

    async fn get_identity_token_balances(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetIdentityTokenBalancesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityTokenBalancesResponse>, Status> {
        Err(Status::unimplemented(
            "get_identity_token_balances not implemented",
        ))
    }

    async fn get_identities_token_balances(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetIdentitiesTokenBalancesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentitiesTokenBalancesResponse>, Status> {
        Err(Status::unimplemented(
            "get_identities_token_balances not implemented",
        ))
    }

    async fn get_identity_token_infos(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetIdentityTokenInfosRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentityTokenInfosResponse>, Status> {
        Err(Status::unimplemented(
            "get_identity_token_infos not implemented",
        ))
    }

    async fn get_identities_token_infos(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetIdentitiesTokenInfosRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetIdentitiesTokenInfosResponse>, Status> {
        Err(Status::unimplemented(
            "get_identities_token_infos not implemented",
        ))
    }

    async fn get_token_statuses(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenStatusesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTokenStatusesResponse>, Status> {
        Err(Status::unimplemented("get_token_statuses not implemented"))
    }

    async fn get_token_direct_purchase_prices(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenDirectPurchasePricesRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTokenDirectPurchasePricesResponse>, Status>
    {
        Err(Status::unimplemented(
            "get_token_direct_purchase_prices not implemented",
        ))
    }

    async fn get_token_contract_info(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenContractInfoRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTokenContractInfoResponse>, Status> {
        Err(Status::unimplemented(
            "get_token_contract_info not implemented",
        ))
    }

    async fn get_token_pre_programmed_distributions(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenPreProgrammedDistributionsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTokenPreProgrammedDistributionsResponse>, Status>
    {
        Err(Status::unimplemented(
            "get_token_pre_programmed_distributions not implemented",
        ))
    }

    async fn get_token_perpetual_distribution_last_claim(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimRequest>,
    ) -> Result<
        Response<dapi_grpc::platform::v0::GetTokenPerpetualDistributionLastClaimResponse>,
        Status,
    > {
        Err(Status::unimplemented(
            "get_token_perpetual_distribution_last_claim not implemented",
        ))
    }

    async fn get_token_total_supply(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetTokenTotalSupplyRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetTokenTotalSupplyResponse>, Status> {
        Err(Status::unimplemented(
            "get_token_total_supply not implemented",
        ))
    }

    async fn get_group_info(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetGroupInfoRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetGroupInfoResponse>, Status> {
        Err(Status::unimplemented("get_group_info not implemented"))
    }

    async fn get_group_infos(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetGroupInfosRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetGroupInfosResponse>, Status> {
        Err(Status::unimplemented("get_group_infos not implemented"))
    }

    async fn get_group_actions(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetGroupActionsRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetGroupActionsResponse>, Status> {
        Err(Status::unimplemented("get_group_actions not implemented"))
    }

    async fn get_group_action_signers(
        &self,
        _request: Request<dapi_grpc::platform::v0::GetGroupActionSignersRequest>,
    ) -> Result<Response<dapi_grpc::platform::v0::GetGroupActionSignersResponse>, Status> {
        Err(Status::unimplemented(
            "get_group_action_signers not implemented",
        ))
    }
}
