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

use crate::config::Config;
use crate::services::streaming_service::StreamingServiceImpl;

/// Core service implementation that handles blockchain and streaming operations
#[derive(Clone)]
pub struct CoreServiceImpl {
    pub streaming_service: Arc<StreamingServiceImpl>,
    pub config: Arc<Config>,
}

impl CoreServiceImpl {
    pub fn new(streaming_service: Arc<StreamingServiceImpl>, config: Arc<Config>) -> Self {
        Self {
            streaming_service,
            config,
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
        _request: Request<GetTransactionRequest>,
    ) -> Result<Response<GetTransactionResponse>, Status> {
        trace!("Received get_transaction request");
        Err(Status::unimplemented("get_transaction not yet implemented"))
    }

    async fn get_best_block_height(
        &self,
        _request: Request<GetBestBlockHeightRequest>,
    ) -> Result<Response<GetBestBlockHeightResponse>, Status> {
        trace!("Received get_best_block_height request");
        Err(Status::unimplemented(
            "get_best_block_height not yet implemented",
        ))
    }

    async fn broadcast_transaction(
        &self,
        _request: Request<BroadcastTransactionRequest>,
    ) -> Result<Response<BroadcastTransactionResponse>, Status> {
        trace!("Received broadcast_transaction request");
        Err(Status::unimplemented(
            "broadcast_transaction not yet implemented",
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        clients::mock::{MockDriveClient, MockTenderdashClient},
        services::streaming_service::StreamingServiceImpl,
    };

    #[tokio::test]
    async fn test_core_service_creation() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());
        let streaming_service = Arc::new(
            StreamingServiceImpl::new(
                drive_client.clone(),
                tenderdash_client.clone(),
                config.clone(),
            )
            .unwrap(),
        );
        let service = CoreServiceImpl::new(streaming_service, config);
        assert!(!service.config.dapi.core.zmq_url.is_empty());
    }

    #[tokio::test]
    async fn test_streaming_service_integration() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());
        let streaming_service = Arc::new(
            StreamingServiceImpl::new(
                drive_client.clone(),
                tenderdash_client.clone(),
                config.clone(),
            )
            .unwrap(),
        );
        let service = CoreServiceImpl::new(streaming_service, config);

        // Test that streaming service is properly initialized
        assert_eq!(
            service
                .streaming_service
                .subscriber_manager
                .subscription_count()
                .await,
            0
        );
    }
}
