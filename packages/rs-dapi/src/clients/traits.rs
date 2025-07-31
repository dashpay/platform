use anyhow::Result;
use async_trait::async_trait;
use dapi_grpc::platform::v0::*;
use std::fmt::Debug;

use super::drive_client::DriveStatusResponse;
use super::tenderdash_client::{
    BroadcastTxResponse, CheckTxResponse, NetInfoResponse, TenderdashStatusResponse, TxResponse,
    UnconfirmedTxsResponse,
};

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
}
