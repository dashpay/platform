use crate::server::query::error::{error_into_status, query_error_into_status};
use async_trait::async_trait;
use dapi_grpc::platform::v0::platform_server::Platform as PlatformService;
use dapi_grpc::platform::v0::{
    BroadcastStateTransitionRequest, BroadcastStateTransitionResponse, GetConsensusParamsRequest,
    GetConsensusParamsResponse, GetDataContractHistoryRequest, GetDataContractHistoryResponse,
    GetDataContractRequest, GetDataContractResponse, GetDataContractsRequest,
    GetDataContractsResponse, GetDocumentsRequest, GetDocumentsResponse, GetEpochsInfoRequest,
    GetEpochsInfoResponse, GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse, GetIdentitiesRequest, GetIdentitiesResponse,
    GetIdentityBalanceAndRevisionRequest, GetIdentityBalanceAndRevisionResponse,
    GetIdentityBalanceRequest, GetIdentityBalanceResponse, GetIdentityByPublicKeyHashRequest,
    GetIdentityByPublicKeyHashResponse, GetIdentityKeysRequest, GetIdentityKeysResponse,
    GetIdentityRequest, GetIdentityResponse, GetProofsRequest, GetProofsResponse,
    GetProtocolVersionUpgradeStateRequest, GetProtocolVersionUpgradeStateResponse,
    GetProtocolVersionUpgradeVoteStatusRequest, GetProtocolVersionUpgradeVoteStatusResponse,
    WaitForStateTransitionResultRequest, WaitForStateTransitionResultResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dpp::version::PlatformVersion;
use dpp::version::PlatformVersionCurrentVersion;
use drive_abci::error::Error;
use drive_abci::platform_types::platform::Platform;
use drive_abci::query::QueryValidationResult;
use drive_abci::rpc::core::DefaultCoreRPC;
use std::sync::Arc;

pub struct QueryServer {
    platform: Arc<Platform<DefaultCoreRPC>>,
}

type QueryMethod<RQ, RS> = fn(
    &Platform<DefaultCoreRPC>,
    request: RQ,
    platform_version: &PlatformVersion,
) -> Result<QueryValidationResult<RS>, Error>;

impl QueryServer {
    pub fn new(platform: Arc<Platform<DefaultCoreRPC>>) -> Self {
        Self { platform }
    }

    fn handle_query<RQ, RS>(
        &self,
        request: Request<RQ>,
        query_method: QueryMethod<RQ, RS>,
    ) -> Result<Response<RS>, Status>
    where
        RS: Clone,
    {
        let Some(platform_version) = PlatformVersion::get_maybe_current() else {
            return Err(Status::unavailable("platform is not initialized"));
        };

        let mut result = query_method(
            self.platform.as_ref(),
            request.into_inner(),
            platform_version,
        )
        .map_err(error_into_status)?;

        if result.is_valid() {
            let response = result
                .into_data()
                .map_err(|error| error_into_status(error.into()))?;

            Ok(Response::new(response))
        } else {
            let error = result.errors.swap_remove(0);

            Err(query_error_into_status(error))
        }
    }
}

fn respond_with_unimplemented<RS>(name: &str) -> Result<Response<RS>, Status> {
    tracing::error!("{} endpoint is called but it's not supported", name);

    Err(Status::unimplemented("the endpoint is not supported"))
}

#[async_trait]
impl PlatformService for QueryServer {
    async fn broadcast_state_transition(
        &self,
        _request: Request<BroadcastStateTransitionRequest>,
    ) -> Result<Response<BroadcastStateTransitionResponse>, Status> {
        respond_with_unimplemented("broadcast state transition")
    }

    async fn get_identity(
        &self,
        request: Request<GetIdentityRequest>,
    ) -> Result<Response<GetIdentityResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_identity)
    }

    async fn get_identities(
        &self,
        request: Request<GetIdentitiesRequest>,
    ) -> Result<Response<GetIdentitiesResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_identities)
    }

    async fn get_identity_keys(
        &self,
        request: Request<GetIdentityKeysRequest>,
    ) -> Result<Response<GetIdentityKeysResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_keys)
    }

    async fn get_identity_balance(
        &self,
        request: Request<GetIdentityBalanceRequest>,
    ) -> Result<Response<GetIdentityBalanceResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_balance)
    }

    async fn get_identity_balance_and_revision(
        &self,
        request: Request<GetIdentityBalanceAndRevisionRequest>,
    ) -> Result<Response<GetIdentityBalanceAndRevisionResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_balance_and_revision,
        )
    }

    async fn get_proofs(
        &self,
        request: Request<GetProofsRequest>,
    ) -> Result<Response<GetProofsResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_proofs)
    }

    async fn get_data_contract(
        &self,
        request: Request<GetDataContractRequest>,
    ) -> Result<Response<GetDataContractResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_data_contract)
    }

    async fn get_data_contract_history(
        &self,
        request: Request<GetDataContractHistoryRequest>,
    ) -> Result<Response<GetDataContractHistoryResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_data_contract_history,
        )
    }

    async fn get_data_contracts(
        &self,
        request: Request<GetDataContractsRequest>,
    ) -> Result<Response<GetDataContractsResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_data_contracts)
    }

    async fn get_documents(
        &self,
        request: Request<GetDocumentsRequest>,
    ) -> Result<Response<GetDocumentsResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_documents)
    }

    async fn get_identities_by_public_key_hashes(
        &self,
        request: Request<GetIdentitiesByPublicKeyHashesRequest>,
    ) -> Result<Response<GetIdentitiesByPublicKeyHashesResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_identities_by_public_key_hashes,
        )
    }

    async fn get_identity_by_public_key_hash(
        &self,
        request: Request<GetIdentityByPublicKeyHashRequest>,
    ) -> Result<Response<GetIdentityByPublicKeyHashResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_identity_by_public_key_hash,
        )
    }

    async fn wait_for_state_transition_result(
        &self,
        _request: Request<WaitForStateTransitionResultRequest>,
    ) -> Result<Response<WaitForStateTransitionResultResponse>, Status> {
        respond_with_unimplemented("wait for state transition result")
    }

    async fn get_consensus_params(
        &self,
        _request: Request<GetConsensusParamsRequest>,
    ) -> Result<Response<GetConsensusParamsResponse>, Status> {
        respond_with_unimplemented("get consensus params")
    }

    async fn get_protocol_version_upgrade_state(
        &self,
        request: Request<GetProtocolVersionUpgradeStateRequest>,
    ) -> Result<Response<GetProtocolVersionUpgradeStateResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_version_upgrade_state,
        )
    }

    async fn get_protocol_version_upgrade_vote_status(
        &self,
        request: Request<GetProtocolVersionUpgradeVoteStatusRequest>,
    ) -> Result<Response<GetProtocolVersionUpgradeVoteStatusResponse>, Status> {
        self.handle_query(
            request,
            Platform::<DefaultCoreRPC>::query_version_upgrade_vote_status,
        )
    }

    async fn get_epochs_info(
        &self,
        request: Request<GetEpochsInfoRequest>,
    ) -> Result<Response<GetEpochsInfoResponse>, Status> {
        self.handle_query(request, Platform::<DefaultCoreRPC>::query_epoch_infos)
    }
}
