// Core service implementation

use dapi_grpc::core::v0::{
    core_server::Core, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
    BroadcastTransactionRequest, BroadcastTransactionResponse, GetBestBlockHeightRequest,
    GetBestBlockHeightResponse, GetBlockRequest, GetBlockResponse, GetBlockchainStatusRequest,
    GetBlockchainStatusResponse, GetEstimatedTransactionFeeRequest,
    GetEstimatedTransactionFeeResponse, GetMasternodeStatusRequest, GetMasternodeStatusResponse,
    GetTransactionRequest, GetTransactionResponse, MasternodeListRequest, MasternodeListResponse,
    TransactionsWithProofsRequest, TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use std::sync::Arc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::trace;

use crate::clients::CoreClient;
use crate::config::Config;
use crate::services::streaming_service::StreamingServiceImpl;

/// Core service implementation that handles blockchain and streaming operations
#[derive(Clone)]
pub struct CoreServiceImpl {
    pub streaming_service: Arc<StreamingServiceImpl>,
    pub config: Arc<Config>,
    pub core_client: CoreClient,
}

impl CoreServiceImpl {
    pub fn new(
        streaming_service: Arc<StreamingServiceImpl>,
        config: Arc<Config>,
        core_client: CoreClient,
    ) -> Self {
        Self { 
            streaming_service, 
            config, 
            core_client 
        }
    }
}

#[dapi_grpc::tonic::async_trait]
impl Core for CoreServiceImpl {
    type subscribeToBlockHeadersWithChainLocksStream =
        UnboundedReceiverStream<Result<BlockHeadersWithChainLocksResponse, Status>>;
    type subscribeToTransactionsWithProofsStream =
        UnboundedReceiverStream<Result<TransactionsWithProofsResponse, Status>>;
    type subscribeToMasternodeListStream =
        UnboundedReceiverStream<Result<MasternodeListResponse, Status>>;

    async fn get_block(
        &self,
        _request: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status> {
        trace!("Received get_block request");
        Err(Status::unimplemented("get_block not yet implemented"))
    }

    async fn get_transaction(
        &self,
        request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        trace!("Received get_transaction request");
        let txid = request.into_inner().id;

        let info = self
            .core_client
            .get_transaction_info(&txid)
            .await
            .map_err(|e| Status::unavailable(e.to_string()))?;

        let transaction = info.hex.clone();
        let block_hash = info
            .blockhash
            .map(|h| h.to_byte_array().to_vec())
            .unwrap_or_default();
        let height = info.height.unwrap_or(0).try_into().unwrap_or(0);
        let confirmations = info.confirmations.unwrap_or(0);
        let is_instant_locked = info.instantlock;
        let is_chain_locked = info.chainlock;

        let response = GetTransactionResponse {
            transaction,
            block_hash,
            height,
            confirmations,
            is_instant_locked,
            is_chain_locked,
        };
        Ok(Response::new(response))
    }

    async fn get_best_block_height(
        &self,
        _request: Request<GetBestBlockHeightRequest>,
    ) -> Result<Response<GetBestBlockHeightResponse>, Status> {
        trace!("Received get_best_block_height request");
        let height = self
            .core_client
            .get_block_count()
            .await
            .map_err(|e| Status::unavailable(e.to_string()))?;

        Ok(Response::new(GetBestBlockHeightResponse { height }))
    }

    async fn broadcast_transaction(
        &self,
        request: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        trace!("Received broadcast_transaction request");
        let req = request.into_inner();
        let _allow_high_fees = req.allow_high_fees;
        let _bypass_limits = req.bypass_limits;

        // NOTE: dashcore-rpc Client does not expose options for allowhighfees/bypasslimits.
        // We broadcast as-is. Future: add support if library exposes those options.
        let txid = self
            .core_client
            .send_raw_transaction(&req.transaction)
            .await
            .map_err(|e| Status::invalid_argument(e.to_string()))?;

        Ok(Response::new(BroadcastTransactionResponse { transaction_id: txid }))
    }

    async fn get_blockchain_status(
        &self,
        _request: Request<GetBlockchainStatusRequest>,
    ) -> Result<Response<GetBlockchainStatusResponse>, Status> {
        trace!("Received get_blockchain_status request");
        Err(Status::unimplemented(
            "get_blockchain_status not yet implemented",
        ))
    }

    async fn get_masternode_status(
        &self,
        _request: Request<GetMasternodeStatusRequest>,
    ) -> Result<Response<GetMasternodeStatusResponse>, Status> {
        trace!("Received get_masternode_status request");
        Err(Status::unimplemented(
            "get_masternode_status not yet implemented",
        ))
    }

    async fn get_estimated_transaction_fee(
        &self,
        _request: Request<GetEstimatedTransactionFeeRequest>,
    ) -> Result<Response<GetEstimatedTransactionFeeResponse>, Status> {
        trace!("Received get_estimated_transaction_fee request");
        Err(Status::unimplemented(
            "get_estimated_transaction_fee not yet implemented",
        ))
    }

    async fn subscribe_to_block_headers_with_chain_locks(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<Response<<Self as Core>::subscribeToBlockHeadersWithChainLocksStream>, Status> {
        trace!("Received subscribe_to_block_headers_with_chain_locks request");
        self.streaming_service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await
    }

    async fn subscribe_to_transactions_with_proofs(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<Response<Self::subscribeToTransactionsWithProofsStream>, Status> {
        trace!("Received subscribe_to_transactions_with_proofs request");
        self.streaming_service
            .subscribe_to_transactions_with_proofs_impl(request)
            .await
    }

    async fn subscribe_to_masternode_list(
        &self,
        request: Request<MasternodeListRequest>,
    ) -> Result<Response<Self::subscribeToMasternodeListStream>, Status> {
        trace!("Received subscribe_to_masternode_list request");
        self.streaming_service
            .subscribe_to_masternode_list_impl(request)
            .await
    }
}
