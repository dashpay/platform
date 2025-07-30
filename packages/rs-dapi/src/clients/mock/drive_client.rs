use anyhow::Result;
use async_trait::async_trait;
use dapi_grpc::platform::v0::*;

use crate::clients::{
    drive_client::{
        DriveChain, DriveProtocol, DriveProtocolVersion, DriveSoftware, DriveStatusResponse,
        DriveTime, DriveVersion,
    },
    traits::DriveClientTrait,
};

#[derive(Debug, Clone)]
pub struct MockDriveClient;

impl MockDriveClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl DriveClientTrait for MockDriveClient {
    async fn get_status(&self, _request: &GetStatusRequest) -> Result<DriveStatusResponse> {
        // Return mock data that matches the test expectations
        Ok(DriveStatusResponse {
            version: Some(DriveVersion {
                software: Some(DriveSoftware {
                    drive: Some("1.1.1".to_string()),
                }),
                protocol: Some(DriveProtocol {
                    drive: Some(DriveProtocolVersion {
                        current: Some(1),
                        latest: Some(2),
                    }),
                }),
            }),
            chain: Some(DriveChain {
                core_chain_locked_height: Some(1000),
            }),
            time: Some(DriveTime {
                block: Some(chrono::Utc::now().timestamp() as u64),
                genesis: Some(1700000000),
                epoch: Some(10),
            }),
        })
    }

    // Identity-related methods
    async fn get_identity(&self, _request: &GetIdentityRequest) -> Result<GetIdentityResponse> {
        Ok(GetIdentityResponse::default())
    }

    async fn get_identity_keys(
        &self,
        _request: &GetIdentityKeysRequest,
    ) -> Result<GetIdentityKeysResponse> {
        Ok(GetIdentityKeysResponse::default())
    }

    async fn get_identities_contract_keys(
        &self,
        _request: &GetIdentitiesContractKeysRequest,
    ) -> Result<GetIdentitiesContractKeysResponse> {
        Ok(GetIdentitiesContractKeysResponse::default())
    }

    async fn get_identity_nonce(
        &self,
        _request: &GetIdentityNonceRequest,
    ) -> Result<GetIdentityNonceResponse> {
        Ok(GetIdentityNonceResponse::default())
    }

    async fn get_identity_contract_nonce(
        &self,
        _request: &GetIdentityContractNonceRequest,
    ) -> Result<GetIdentityContractNonceResponse> {
        Ok(GetIdentityContractNonceResponse::default())
    }

    async fn get_identity_balance(
        &self,
        _request: &GetIdentityBalanceRequest,
    ) -> Result<GetIdentityBalanceResponse> {
        Ok(GetIdentityBalanceResponse::default())
    }

    async fn get_identities_balances(
        &self,
        _request: &GetIdentitiesBalancesRequest,
    ) -> Result<GetIdentitiesBalancesResponse> {
        Ok(GetIdentitiesBalancesResponse::default())
    }

    async fn get_identity_balance_and_revision(
        &self,
        _request: &GetIdentityBalanceAndRevisionRequest,
    ) -> Result<GetIdentityBalanceAndRevisionResponse> {
        Ok(GetIdentityBalanceAndRevisionResponse::default())
    }

    async fn get_identity_by_public_key_hash(
        &self,
        _request: &GetIdentityByPublicKeyHashRequest,
    ) -> Result<GetIdentityByPublicKeyHashResponse> {
        Ok(GetIdentityByPublicKeyHashResponse::default())
    }

    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        _request: &GetIdentityByNonUniquePublicKeyHashRequest,
    ) -> Result<GetIdentityByNonUniquePublicKeyHashResponse> {
        Ok(GetIdentityByNonUniquePublicKeyHashResponse::default())
    }

    // Data Contract methods
    async fn get_data_contract(
        &self,
        _request: &GetDataContractRequest,
    ) -> Result<GetDataContractResponse> {
        Ok(GetDataContractResponse::default())
    }

    async fn get_data_contracts(
        &self,
        _request: &GetDataContractsRequest,
    ) -> Result<GetDataContractsResponse> {
        Ok(GetDataContractsResponse::default())
    }

    async fn get_data_contract_history(
        &self,
        _request: &GetDataContractHistoryRequest,
    ) -> Result<GetDataContractHistoryResponse> {
        Ok(GetDataContractHistoryResponse::default())
    }

    // Document methods
    async fn get_documents(&self, _request: &GetDocumentsRequest) -> Result<GetDocumentsResponse> {
        Ok(GetDocumentsResponse::default())
    }

    // Epoch and consensus methods
    async fn get_epochs_info(
        &self,
        _request: &GetEpochsInfoRequest,
    ) -> Result<GetEpochsInfoResponse> {
        Ok(GetEpochsInfoResponse::default())
    }

    async fn get_finalized_epoch_infos(
        &self,
        _request: &GetFinalizedEpochInfosRequest,
    ) -> Result<GetFinalizedEpochInfosResponse> {
        Ok(GetFinalizedEpochInfosResponse::default())
    }

    async fn get_consensus_params(
        &self,
        _request: &GetConsensusParamsRequest,
    ) -> Result<GetConsensusParamsResponse> {
        Ok(GetConsensusParamsResponse::default())
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        _request: &GetProtocolVersionUpgradeStateRequest,
    ) -> Result<GetProtocolVersionUpgradeStateResponse> {
        Ok(GetProtocolVersionUpgradeStateResponse::default())
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        _request: &GetProtocolVersionUpgradeVoteStatusRequest,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusResponse> {
        Ok(GetProtocolVersionUpgradeVoteStatusResponse::default())
    }

    // Other methods
    async fn get_path_elements(
        &self,
        _request: &GetPathElementsRequest,
    ) -> Result<GetPathElementsResponse> {
        Ok(GetPathElementsResponse::default())
    }

    async fn get_total_credits_in_platform(
        &self,
        _request: &GetTotalCreditsInPlatformRequest,
    ) -> Result<GetTotalCreditsInPlatformResponse> {
        Ok(GetTotalCreditsInPlatformResponse::default())
    }

    async fn get_current_quorums_info(
        &self,
        _request: &GetCurrentQuorumsInfoRequest,
    ) -> Result<GetCurrentQuorumsInfoResponse> {
        Ok(GetCurrentQuorumsInfoResponse::default())
    }

    // State transition methods
    async fn broadcast_state_transition(
        &self,
        _request: &BroadcastStateTransitionRequest,
    ) -> Result<BroadcastStateTransitionResponse> {
        Ok(BroadcastStateTransitionResponse::default())
    }

    async fn wait_for_state_transition_result(
        &self,
        _request: &WaitForStateTransitionResultRequest,
    ) -> Result<WaitForStateTransitionResultResponse> {
        Ok(WaitForStateTransitionResultResponse::default())
    }
}
