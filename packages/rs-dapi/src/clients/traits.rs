use async_trait::async_trait;
use dapi_grpc::platform::v0::*;
use std::fmt::Debug;
use tokio::sync::broadcast;

use super::drive_client::DriveStatusResponse;
use super::tenderdash_client::{
    BroadcastTxResponse, CheckTxResponse, NetInfoResponse, TenderdashStatusResponse, TxResponse,
    UnconfirmedTxsResponse,
};
use super::tenderdash_websocket::TransactionEvent;
use crate::error::DAPIResult;

#[async_trait]
pub trait DriveClientTrait: Send + Sync + Debug {
    async fn get_status(&self, request: &GetStatusRequest) -> DAPIResult<DriveStatusResponse>;

    // Identity-related methods
    async fn get_identity(&self, request: &GetIdentityRequest) -> DAPIResult<GetIdentityResponse>;
    async fn get_identity_keys(
        &self,
        request: &GetIdentityKeysRequest,
    ) -> DAPIResult<GetIdentityKeysResponse>;
    async fn get_identities_contract_keys(
        &self,
        request: &GetIdentitiesContractKeysRequest,
    ) -> DAPIResult<GetIdentitiesContractKeysResponse>;
    async fn get_identity_nonce(
        &self,
        request: &GetIdentityNonceRequest,
    ) -> DAPIResult<GetIdentityNonceResponse>;
    async fn get_identity_contract_nonce(
        &self,
        request: &GetIdentityContractNonceRequest,
    ) -> DAPIResult<GetIdentityContractNonceResponse>;
    async fn get_identity_balance(
        &self,
        request: &GetIdentityBalanceRequest,
    ) -> DAPIResult<GetIdentityBalanceResponse>;
    async fn get_identities_balances(
        &self,
        request: &GetIdentitiesBalancesRequest,
    ) -> DAPIResult<GetIdentitiesBalancesResponse>;
    async fn get_identity_balance_and_revision(
        &self,
        request: &GetIdentityBalanceAndRevisionRequest,
    ) -> DAPIResult<GetIdentityBalanceAndRevisionResponse>;
    async fn get_identity_by_public_key_hash(
        &self,
        request: &GetIdentityByPublicKeyHashRequest,
    ) -> DAPIResult<GetIdentityByPublicKeyHashResponse>;
    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: &GetIdentityByNonUniquePublicKeyHashRequest,
    ) -> DAPIResult<GetIdentityByNonUniquePublicKeyHashResponse>;

    // Data Contract methods
    async fn get_data_contract(
        &self,
        request: &GetDataContractRequest,
    ) -> DAPIResult<GetDataContractResponse>;
    async fn get_data_contracts(
        &self,
        request: &GetDataContractsRequest,
    ) -> DAPIResult<GetDataContractsResponse>;
    async fn get_data_contract_history(
        &self,
        request: &GetDataContractHistoryRequest,
    ) -> DAPIResult<GetDataContractHistoryResponse>;

    // Document methods
    async fn get_documents(
        &self,
        request: &GetDocumentsRequest,
    ) -> DAPIResult<GetDocumentsResponse>;

    // Epoch and consensus methods
    async fn get_epochs_info(
        &self,
        request: &GetEpochsInfoRequest,
    ) -> DAPIResult<GetEpochsInfoResponse>;
    async fn get_finalized_epoch_infos(
        &self,
        request: &GetFinalizedEpochInfosRequest,
    ) -> DAPIResult<GetFinalizedEpochInfosResponse>;
    async fn get_consensus_params(
        &self,
        request: &GetConsensusParamsRequest,
    ) -> DAPIResult<GetConsensusParamsResponse>;
    async fn get_protocol_version_upgrade_state(
        &self,
        request: &GetProtocolVersionUpgradeStateRequest,
    ) -> DAPIResult<GetProtocolVersionUpgradeStateResponse>;
    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: &GetProtocolVersionUpgradeVoteStatusRequest,
    ) -> DAPIResult<GetProtocolVersionUpgradeVoteStatusResponse>;

    // Other methods
    async fn get_path_elements(
        &self,
        request: &GetPathElementsRequest,
    ) -> DAPIResult<GetPathElementsResponse>;
    async fn get_total_credits_in_platform(
        &self,
        request: &GetTotalCreditsInPlatformRequest,
    ) -> DAPIResult<GetTotalCreditsInPlatformResponse>;
    async fn get_current_quorums_info(
        &self,
        request: &GetCurrentQuorumsInfoRequest,
    ) -> DAPIResult<GetCurrentQuorumsInfoResponse>;

    // Contested resource methods
    async fn get_contested_resources(
        &self,
        request: &GetContestedResourcesRequest,
    ) -> DAPIResult<GetContestedResourcesResponse>;
    async fn get_contested_resource_vote_state(
        &self,
        request: &GetContestedResourceVoteStateRequest,
    ) -> DAPIResult<GetContestedResourceVoteStateResponse>;
    async fn get_contested_resource_voters_for_identity(
        &self,
        request: &GetContestedResourceVotersForIdentityRequest,
    ) -> DAPIResult<GetContestedResourceVotersForIdentityResponse>;
    async fn get_contested_resource_identity_votes(
        &self,
        request: &GetContestedResourceIdentityVotesRequest,
    ) -> DAPIResult<GetContestedResourceIdentityVotesResponse>;
    async fn get_vote_polls_by_end_date(
        &self,
        request: &GetVotePollsByEndDateRequest,
    ) -> DAPIResult<GetVotePollsByEndDateResponse>;

    // Token methods
    async fn get_identity_token_balances(
        &self,
        request: &GetIdentityTokenBalancesRequest,
    ) -> DAPIResult<GetIdentityTokenBalancesResponse>;
    async fn get_identities_token_balances(
        &self,
        request: &GetIdentitiesTokenBalancesRequest,
    ) -> DAPIResult<GetIdentitiesTokenBalancesResponse>;
    async fn get_identity_token_infos(
        &self,
        request: &GetIdentityTokenInfosRequest,
    ) -> DAPIResult<GetIdentityTokenInfosResponse>;
    async fn get_identities_token_infos(
        &self,
        request: &GetIdentitiesTokenInfosRequest,
    ) -> DAPIResult<GetIdentitiesTokenInfosResponse>;
    async fn get_token_statuses(
        &self,
        request: &GetTokenStatusesRequest,
    ) -> DAPIResult<GetTokenStatusesResponse>;
    async fn get_token_direct_purchase_prices(
        &self,
        request: &GetTokenDirectPurchasePricesRequest,
    ) -> DAPIResult<GetTokenDirectPurchasePricesResponse>;
    async fn get_token_contract_info(
        &self,
        request: &GetTokenContractInfoRequest,
    ) -> DAPIResult<GetTokenContractInfoResponse>;
    async fn get_token_pre_programmed_distributions(
        &self,
        request: &GetTokenPreProgrammedDistributionsRequest,
    ) -> DAPIResult<GetTokenPreProgrammedDistributionsResponse>;
    async fn get_token_perpetual_distribution_last_claim(
        &self,
        request: &GetTokenPerpetualDistributionLastClaimRequest,
    ) -> DAPIResult<GetTokenPerpetualDistributionLastClaimResponse>;
    async fn get_token_total_supply(
        &self,
        request: &GetTokenTotalSupplyRequest,
    ) -> DAPIResult<GetTokenTotalSupplyResponse>;
    async fn get_prefunded_specialized_balance(
        &self,
        request: &GetPrefundedSpecializedBalanceRequest,
    ) -> DAPIResult<GetPrefundedSpecializedBalanceResponse>;

    // Group methods
    async fn get_group_info(
        &self,
        request: &GetGroupInfoRequest,
    ) -> DAPIResult<GetGroupInfoResponse>;
    async fn get_group_infos(
        &self,
        request: &GetGroupInfosRequest,
    ) -> DAPIResult<GetGroupInfosResponse>;
    async fn get_group_actions(
        &self,
        request: &GetGroupActionsRequest,
    ) -> DAPIResult<GetGroupActionsResponse>;
    async fn get_group_action_signers(
        &self,
        request: &GetGroupActionSignersRequest,
    ) -> DAPIResult<GetGroupActionSignersResponse>;

    // State transition methods
    async fn broadcast_state_transition(
        &self,
        request: &BroadcastStateTransitionRequest,
    ) -> DAPIResult<BroadcastStateTransitionResponse>;
    async fn wait_for_state_transition_result(
        &self,
        request: &WaitForStateTransitionResultRequest,
    ) -> DAPIResult<WaitForStateTransitionResultResponse>;
}

#[async_trait]
pub trait TenderdashClientTrait: Send + Sync + Debug {
    async fn status(&self) -> DAPIResult<TenderdashStatusResponse>;
    async fn net_info(&self) -> DAPIResult<NetInfoResponse>;

    // State transition broadcasting methods
    async fn broadcast_tx(&self, tx: String) -> DAPIResult<BroadcastTxResponse>;
    async fn check_tx(&self, tx: String) -> DAPIResult<CheckTxResponse>;
    async fn unconfirmed_txs(&self, limit: Option<u32>) -> DAPIResult<UnconfirmedTxsResponse>;
    async fn tx(&self, hash: String) -> DAPIResult<TxResponse>;

    // WebSocket functionality for waitForStateTransitionResult
    fn subscribe_to_transactions(&self) -> broadcast::Receiver<TransactionEvent>;
    fn is_websocket_connected(&self) -> bool;
}
