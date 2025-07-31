use anyhow::Result;
use async_trait::async_trait;
use dapi_grpc::platform::v0::{
    platform_client::PlatformClient,
    BroadcastStateTransitionRequest,
    BroadcastStateTransitionResponse,
    GetConsensusParamsRequest,
    GetConsensusParamsResponse,
    GetCurrentQuorumsInfoRequest,
    GetCurrentQuorumsInfoResponse,
    GetDataContractHistoryRequest,
    GetDataContractHistoryResponse,
    GetDataContractRequest,
    GetDataContractResponse,
    GetDataContractsRequest,
    GetDataContractsResponse,
    GetDocumentsRequest,
    GetDocumentsResponse,
    GetEpochsInfoRequest,
    GetEpochsInfoResponse,
    GetFinalizedEpochInfosRequest,
    GetFinalizedEpochInfosResponse,
    GetIdentitiesBalancesRequest,
    GetIdentitiesBalancesResponse,
    GetIdentitiesContractKeysRequest,
    GetIdentitiesContractKeysResponse,
    GetIdentityBalanceAndRevisionRequest,
    GetIdentityBalanceAndRevisionResponse,
    GetIdentityBalanceRequest,
    GetIdentityBalanceResponse,
    GetIdentityByNonUniquePublicKeyHashRequest,
    GetIdentityByNonUniquePublicKeyHashResponse,
    GetIdentityByPublicKeyHashRequest,
    GetIdentityByPublicKeyHashResponse,
    GetIdentityContractNonceRequest,
    GetIdentityContractNonceResponse,
    GetIdentityKeysRequest,
    GetIdentityKeysResponse,
    GetIdentityNonceRequest,
    GetIdentityNonceResponse,
    // Import all necessary request/response types
    GetIdentityRequest,
    GetIdentityResponse,
    GetPathElementsRequest,
    GetPathElementsResponse,
    GetProtocolVersionUpgradeStateRequest,
    GetProtocolVersionUpgradeStateResponse,
    GetProtocolVersionUpgradeVoteStatusRequest,
    GetProtocolVersionUpgradeVoteStatusResponse,
    GetStatusRequest,
    GetTotalCreditsInPlatformRequest,
    GetTotalCreditsInPlatformResponse,
    WaitForStateTransitionResultRequest,
    WaitForStateTransitionResultResponse,
};
use serde::{Deserialize, Serialize};
use tracing::{error, info, trace};

use super::traits::DriveClientTrait;

#[derive(Debug, Clone)]
pub struct DriveClient {
    base_url: String,
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

impl DriveClient {
    pub fn new(uri: &str) -> Self {
        info!("Creating Drive client for: {}", uri);
        Self {
            base_url: uri.to_string(),
        }
    }

    pub async fn get_status(&self, request: &GetStatusRequest) -> Result<DriveStatusResponse> {
        trace!("Connecting to Drive service at: {}", self.base_url);
        // Attempt to connect to Drive gRPC service
        let mut client = match dapi_grpc::platform::v0::platform_client::PlatformClient::connect(
            self.base_url.clone(),
        )
        .await
        {
            Ok(client) => {
                trace!("Successfully connected to Drive service");
                client
            },
            Err(e) => {
                error!("Failed to connect to Drive service at {}: {}", self.base_url, e);
                return Err(anyhow::anyhow!(
                    "Failed to connect to Drive service at {}: {}",
                    self.base_url,
                    e
                ));
            }
        };

        trace!("Making get_status gRPC call to Drive");
        // Make gRPC call to Drive
        let response = client
            .get_status(dapi_grpc::tonic::Request::new(*request))
            .await?;
        let drive_response = response.into_inner();

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
            Err(anyhow::anyhow!("Drive returned unexpected response format"))
        }
    }
}

#[async_trait]
impl DriveClientTrait for DriveClient {
    async fn get_status(&self, request: &GetStatusRequest) -> Result<DriveStatusResponse> {
        self.get_status(request).await
    }

    // Identity-related methods
    async fn get_identity(&self, request: &GetIdentityRequest) -> Result<GetIdentityResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_keys(
        &self,
        request: &GetIdentityKeysRequest,
    ) -> Result<GetIdentityKeysResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_keys(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_contract_keys(
        &self,
        request: &GetIdentitiesContractKeysRequest,
    ) -> Result<GetIdentitiesContractKeysResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_contract_keys(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_nonce(
        &self,
        request: &GetIdentityNonceRequest,
    ) -> Result<GetIdentityNonceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_nonce(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_contract_nonce(
        &self,
        request: &GetIdentityContractNonceRequest,
    ) -> Result<GetIdentityContractNonceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_contract_nonce(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_balance(
        &self,
        request: &GetIdentityBalanceRequest,
    ) -> Result<GetIdentityBalanceResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_balance(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identities_balances(
        &self,
        request: &GetIdentitiesBalancesRequest,
    ) -> Result<GetIdentitiesBalancesResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identities_balances(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_balance_and_revision(
        &self,
        request: &GetIdentityBalanceAndRevisionRequest,
    ) -> Result<GetIdentityBalanceAndRevisionResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_balance_and_revision(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_by_public_key_hash(
        &self,
        request: &GetIdentityByPublicKeyHashRequest,
    ) -> Result<GetIdentityByPublicKeyHashResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_identity_by_public_key_hash(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_identity_by_non_unique_public_key_hash(
        &self,
        request: &GetIdentityByNonUniquePublicKeyHashRequest,
    ) -> Result<GetIdentityByNonUniquePublicKeyHashResponse> {
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
    ) -> Result<GetDataContractResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contract(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_data_contracts(
        &self,
        request: &GetDataContractsRequest,
    ) -> Result<GetDataContractsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contracts(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_data_contract_history(
        &self,
        request: &GetDataContractHistoryRequest,
    ) -> Result<GetDataContractHistoryResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_data_contract_history(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // Document methods
    async fn get_documents(&self, request: &GetDocumentsRequest) -> Result<GetDocumentsResponse> {
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
    ) -> Result<GetEpochsInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_epochs_info(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_finalized_epoch_infos(
        &self,
        request: &GetFinalizedEpochInfosRequest,
    ) -> Result<GetFinalizedEpochInfosResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_finalized_epoch_infos(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_consensus_params(
        &self,
        request: &GetConsensusParamsRequest,
    ) -> Result<GetConsensusParamsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_consensus_params(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        request: &GetProtocolVersionUpgradeStateRequest,
    ) -> Result<GetProtocolVersionUpgradeStateResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_protocol_version_upgrade_state(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: &GetProtocolVersionUpgradeVoteStatusRequest,
    ) -> Result<GetProtocolVersionUpgradeVoteStatusResponse> {
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
    ) -> Result<GetPathElementsResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_path_elements(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_total_credits_in_platform(
        &self,
        request: &GetTotalCreditsInPlatformRequest,
    ) -> Result<GetTotalCreditsInPlatformResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_total_credits_in_platform(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn get_current_quorums_info(
        &self,
        request: &GetCurrentQuorumsInfoRequest,
    ) -> Result<GetCurrentQuorumsInfoResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .get_current_quorums_info(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    // State transition methods
    async fn broadcast_state_transition(
        &self,
        request: &BroadcastStateTransitionRequest,
    ) -> Result<BroadcastStateTransitionResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .broadcast_state_transition(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }

    async fn wait_for_state_transition_result(
        &self,
        request: &WaitForStateTransitionResultRequest,
    ) -> Result<WaitForStateTransitionResultResponse> {
        let mut client = self.get_client().await?;
        let response = client
            .wait_for_state_transition_result(dapi_grpc::tonic::Request::new(request.clone()))
            .await?;
        Ok(response.into_inner())
    }
}

impl DriveClient {
    // Helper method to get a connected client
    async fn get_client(&self) -> Result<PlatformClient<dapi_grpc::tonic::transport::Channel>> {
        match PlatformClient::connect(self.base_url.clone()).await {
            Ok(client) => Ok(client),
            Err(e) => Err(anyhow::anyhow!(
                "Failed to connect to Platform service at {}: {}",
                self.base_url,
                e
            )),
        }
    }
}
