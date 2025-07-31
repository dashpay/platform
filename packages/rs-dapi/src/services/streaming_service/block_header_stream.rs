use dapi_grpc::core::v0::{
    BlockHeaders, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info};

use crate::services::streaming_service::{
    FilterType, StreamingMessage, StreamingServiceImpl, SubscriptionType,
};

impl StreamingServiceImpl {
    pub async fn subscribe_to_block_headers_with_chain_locks_impl(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<
        Response<UnboundedReceiverStream<Result<BlockHeadersWithChainLocksResponse, Status>>>,
        Status,
    > {
        let req = request.into_inner();

        // Validate parameters
        let count = req.count;
        let from_block = req.from_block.clone();

        // Validate that we have from_block when count > 0
        if from_block.is_none() && count > 0 {
            return Err(Status::invalid_argument(
                "Must specify from_block when count > 0",
            ));
        }

        // Create filter (no filtering needed for block headers - all blocks)
        let filter = FilterType::AllBlocks;

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // Create message channel for internal communication
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<StreamingMessage>();

        // Add subscription to manager
        let subscription_id = self
            .subscriber_manager
            .add_subscription(
                filter,
                SubscriptionType::BlockHeadersWithChainLocks,
                message_tx,
            )
            .await;

        info!("Started block header subscription: {}", subscription_id);

        // Spawn task to convert internal messages to gRPC responses
        let subscriber_manager = self.subscriber_manager.clone();
        let sub_id = subscription_id.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                let response = match message {
                    StreamingMessage::BlockHeader { data } => {
                        let mut block_headers = BlockHeaders::default();
                        block_headers.headers = vec![data];

                        let mut response = BlockHeadersWithChainLocksResponse::default();
                        response.responses = Some(
                            dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(block_headers)
                        );

                        Ok(response)
                    }
                    StreamingMessage::ChainLock { data } => {
                        let mut response = BlockHeadersWithChainLocksResponse::default();
                        response.responses = Some(
                            dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::ChainLock(data)
                        );

                        Ok(response)
                    }
                    _ => {
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if let Err(_) = tx.send(response) {
                    debug!(
                        "Client disconnected from block header subscription: {}",
                        sub_id
                    );
                    break;
                }
            }

            // Clean up subscription when client disconnects
            subscriber_manager.remove_subscription(&sub_id).await;
            info!("Cleaned up block header subscription: {}", sub_id);
        });

        // Handle historical data if requested
        if count > 0 {
            if let Some(from_block) = from_block {
                match from_block {
                    dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHash(hash) => {
                        // TODO: Process historical block headers from block hash
                        debug!(
                            "Historical block header processing requested from hash: {:?}",
                            hash
                        );
                        self.process_historical_blocks_from_hash(&hash, count as usize)
                            .await?;
                    }
                    dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(height) => {
                        // TODO: Process historical block headers from height
                        debug!(
                            "Historical block header processing requested from height: {}",
                            height
                        );
                        self.process_historical_blocks_from_height(
                            height as usize,
                            count as usize,
                        )
                        .await?;
                    }
                }
            }
        }

        let stream = UnboundedReceiverStream::new(rx);
        Ok(Response::new(stream))
    }

    /// Process historical blocks from a specific block hash
    async fn process_historical_blocks_from_hash(
        &self,
        _from_hash: &[u8],
        _count: usize,
    ) -> Result<(), Status> {
        // TODO: Implement historical block processing from hash
        // This should:
        // 1. Look up the block height for the given hash
        // 2. Fetch the requested number of blocks starting from that height
        // 3. Send block headers to the subscriber
        debug!("Processing historical blocks from hash not yet implemented");
        Ok(())
    }

    /// Process historical blocks from a specific block height
    async fn process_historical_blocks_from_height(
        &self,
        _from_height: usize,
        _count: usize,
    ) -> Result<(), Status> {
        // TODO: Implement historical block processing from height
        // This should:
        // 1. Fetch blocks starting from the given height
        // 2. Extract block headers
        // 3. Send headers to the subscriber
        // 4. Include any available chain locks
        debug!("Processing historical blocks from height not yet implemented");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::mock::{MockDriveClient, MockTenderdashClient};
    use crate::config::Config;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_block_header_subscription_creation() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());

        let service = StreamingServiceImpl::new(drive_client, tenderdash_client, config).unwrap();

        let request = Request::new(BlockHeadersWithChainLocksRequest {
            from_block: Some(
                dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(100)
            ),
            count: 0, // Streaming mode
        });

        let result = service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_block_header_subscription_with_historical() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());

        let service = StreamingServiceImpl::new(drive_client, tenderdash_client, config).unwrap();

        let request = Request::new(BlockHeadersWithChainLocksRequest {
            from_block: Some(
                dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(100)
            ),
            count: 10, // Get 10 historical blocks
        });

        let result = service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_block_header_subscription_invalid_params() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());

        let service = StreamingServiceImpl::new(drive_client, tenderdash_client, config).unwrap();

        let request = Request::new(BlockHeadersWithChainLocksRequest {
            from_block: None, // No from_block specified
            count: 10,        // But requesting historical data
        });

        let result = service
            .subscribe_to_block_headers_with_chain_locks_impl(request)
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code(),
            dapi_grpc::tonic::Code::InvalidArgument
        );
    }
}
