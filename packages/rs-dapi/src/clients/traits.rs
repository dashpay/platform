use anyhow::Result;
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

#[async_trait]
pub trait DriveClientTrait: Send + Sync + Debug {
    async fn get_status(&self, request: &GetStatusRequest) -> Result<DriveStatusResponse>;

    // Identity-related methods
    async fn get_identity(&self, request: &GetIdentityRequest) -> Result<GetIdentityResponse>;
    async fn get_identity_keys(
        &self,
        request: &GetIdentityKeysRequest,
    ) -> Result<GetIdentityKeysResponse>;
    async fn get_identities_contract_keys(
        &self,
        request: &GetIdentitiesContractKeysRequest,
    ) -> Result<GetIdentitiesContractKeysResponse>;
    async fn get_identity_nonce(
        &self,
        request: &GetIdentityNonceRequest,
    ) -> Result<GetIdentityNonceResponse>;
    async fn get_identity_contract_nonce(
        &self,
        request: &GetIdentityContractNonceRequest,
    ) -> Result<GetIdentityContractNonceResponse>;
    async fn get_identity_balance(
        &self,
        request: &GetIdentityBalanceRequest,
    ) -> Result<GetIdentityBalanceResponse>;
    async fn get_identities_balances(
        &self,
        request: &GetIdentitiesBalancesRequest,
    ) -> Result<GetIdentitiesBalancesResponse>;
    async fn get_identity_balance_and_revision(
        &self,
        request: &GetIdentityBalanceAndRevisionRequest,
    ) -> Result<GetIdentityBalanceAndRevisionResponse>;
    async fn get_identity_by_public_key_hash(
        &self,
        request: &GetIdentityByPublicKeyHashRequest,
    ) -> Result<GetIdentityByPublicKeyHashResponse>;
    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: &GetIdentityByNonUniquePublicKeyHashRequest,
    ) -> Result<GetIdentityByNonUniquePublicKeyHashResponse>;

    // Data Contract methods
    async fn get_data_contract(
        &self,
        request: &GetDataContractRequest,
    ) -> Result<GetDataContractResponse>;
    async fn get_data_contracts(
        &self,
        request: &GetDataContractsRequest,
    ) -> Result<GetDataContractsResponse>;
    async fn get_data_contract_history(
        &self,
        request: &GetDataContractHistoryRequest,
    ) -> Result<GetDataContractHistoryResponse>;

    // Document methods
    async fn get_documents(&self, request: &GetDocumentsRequest) -> Result<GetDocumentsResponse>;

    // Epoch and consensus methods
    async fn get_epochs_info(
        &self,
        request: &GetEpochsInfoRequest,
    ) -> Result<GetEpochsInfoResponse>;
    async fn get_finalized_epoch_infos(
        &self,
        request: &GetFinalizedEpochInfosRequest,
    ) -> Result<GetFinalizedEpochInfosResponse>;
    async fn get_consensus_params(
        &self,
        request: &GetConsensusParamsRequest,
    ) -> Result<GetConsensusParamsResponse>;
    async fn get_protocol_version_upgrade_state(
        &self,
        request: &GetProtocolVersionUpgradeStateRequest,
    ) -> Result<GetProtocolVersionUpgradeStateResponse>;
    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: &GetProtocolVersionUpgradeVoteStatusRequest,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusResponse>;

    // Other methods
    async fn get_path_elements(
        &self,
        request: &GetPathElementsRequest,
    ) -> Result<GetPathElementsResponse>;
    async fn get_total_credits_in_platform(
        &self,
        request: &GetTotalCreditsInPlatformRequest,
    ) -> Result<GetTotalCreditsInPlatformResponse>;
    async fn get_current_quorums_info(
        &self,
        request: &GetCurrentQuorumsInfoRequest,
    ) -> Result<GetCurrentQuorumsInfoResponse>;

    // Contested resource methods
    async fn get_contested_resources(
        &self,
        request: &GetContestedResourcesRequest,
    ) -> Result<GetContestedResourcesResponse>;
    async fn get_contested_resource_vote_state(
        &self,
        request: &GetContestedResourceVoteStateRequest,
    ) -> Result<GetContestedResourceVoteStateResponse>;
    async fn get_contested_resource_voters_for_identity(
        &self,
        request: &GetContestedResourceVotersForIdentityRequest,
    ) -> Result<GetContestedResourceVotersForIdentityResponse>;
    async fn get_contested_resource_identity_votes(
        &self,
        request: &GetContestedResourceIdentityVotesRequest,
    ) -> Result<GetContestedResourceIdentityVotesResponse>;
    async fn get_vote_polls_by_end_date(
        &self,
        request: &GetVotePollsByEndDateRequest,
    ) -> Result<GetVotePollsByEndDateResponse>;

    // Token methods
    async fn get_identity_token_balances(
        &self,
        request: &GetIdentityTokenBalancesRequest,
    ) -> Result<GetIdentityTokenBalancesResponse>;
    async fn get_identities_token_balances(
        &self,
        request: &GetIdentitiesTokenBalancesRequest,
    ) -> Result<GetIdentitiesTokenBalancesResponse>;
    async fn get_identity_token_infos(
        &self,
        request: &GetIdentityTokenInfosRequest,
    ) -> Result<GetIdentityTokenInfosResponse>;
    async fn get_identities_token_infos(
        &self,
        request: &GetIdentitiesTokenInfosRequest,
    ) -> Result<GetIdentitiesTokenInfosResponse>;
    async fn get_token_statuses(
        &self,
        request: &GetTokenStatusesRequest,
    ) -> Result<GetTokenStatusesResponse>;
    async fn get_token_direct_purchase_prices(
        &self,
        request: &GetTokenDirectPurchasePricesRequest,
    ) -> Result<GetTokenDirectPurchasePricesResponse>;
    async fn get_token_contract_info(
        &self,
        request: &GetTokenContractInfoRequest,
    ) -> Result<GetTokenContractInfoResponse>;
    async fn get_token_pre_programmed_distributions(
        &self,
        request: &GetTokenPreProgrammedDistributionsRequest,
    ) -> Result<GetTokenPreProgrammedDistributionsResponse>;
    async fn get_token_perpetual_distribution_last_claim(
        &self,
        request: &GetTokenPerpetualDistributionLastClaimRequest,
    ) -> Result<GetTokenPerpetualDistributionLastClaimResponse>;
    async fn get_token_total_supply(
        &self,
        request: &GetTokenTotalSupplyRequest,
    ) -> Result<GetTokenTotalSupplyResponse>;
    async fn get_prefunded_specialized_balance(
        &self,
        request: &GetPrefundedSpecializedBalanceRequest,
    ) -> Result<GetPrefundedSpecializedBalanceResponse>;

    // Group methods
    async fn get_group_info(&self, request: &GetGroupInfoRequest) -> Result<GetGroupInfoResponse>;
    async fn get_group_infos(
        &self,
        request: &GetGroupInfosRequest,
    ) -> Result<GetGroupInfosResponse>;
    async fn get_group_actions(
        &self,
        request: &GetGroupActionsRequest,
    ) -> Result<GetGroupActionsResponse>;
    async fn get_group_action_signers(
        &self,
        request: &GetGroupActionSignersRequest,
    ) -> Result<GetGroupActionSignersResponse>;

    // State transition methods
    async fn broadcast_state_transition(
        &self,
        request: &BroadcastStateTransitionRequest,
    ) -> Result<BroadcastStateTransitionResponse>;
    async fn wait_for_state_transition_result(
        &self,
        request: &WaitForStateTransitionResultRequest,
    ) -> Result<WaitForStateTransitionResultResponse>;
}

#[async_trait]
pub trait TenderdashClientTrait: Send + Sync + Debug {
    async fn status(&self) -> Result<TenderdashStatusResponse>;
    async fn net_info(&self) -> Result<NetInfoResponse>;

    // State transition broadcasting methods
    async fn broadcast_tx(&self, tx: String) -> Result<BroadcastTxResponse>;
    async fn check_tx(&self, tx: String) -> Result<CheckTxResponse>;
    async fn unconfirmed_txs(&self, limit: Option<u32>) -> Result<UnconfirmedTxsResponse>;
    async fn tx(&self, hash: String) -> Result<TxResponse>;

    // WebSocket functionality for waitForStateTransitionResult
    fn subscribe_to_transactions(&self) -> broadcast::Receiver<TransactionEvent>;
    fn is_websocket_connected(&self) -> bool;
}
