use dapi_grpc::core::v0::{
    BlockHeaders, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, trace, warn};

use crate::services::streaming_service::{FilterType, StreamingEvent, StreamingServiceImpl};

impl StreamingServiceImpl {
    pub async fn subscribe_to_block_headers_with_chain_locks_impl(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<
        Response<UnboundedReceiverStream<Result<BlockHeadersWithChainLocksResponse, Status>>>,
        Status,
    > {
        trace!("subscribe_to_block_headers_with_chain_locks_impl=begin");
        let req = request.into_inner();

        // Validate parameters
        let count = req.count;
        let from_block = req.from_block.clone();

        trace!(
            count,
            has_from_block = from_block.is_some(),
            "block_headers=request_parsed"
        );

        // Validate that we have from_block when count > 0
        if from_block.is_none() && count > 0 {
            warn!("block_headers=missing_from_block count>0");
            return Err(Status::invalid_argument(
                "Must specify from_block when count > 0",
            ));
        }

        // Create filter (no filtering needed for block headers - all blocks)
        let filter = FilterType::CoreAllBlocks;

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // Add subscription to manager
        let subscription_handle = self.subscriber_manager.add_subscription(filter).await;
        let subscriber_id = subscription_handle.id().to_string();
        debug!(subscriber_id, "block_headers=subscription_created");

        // Spawn task to convert internal messages to gRPC responses
        let sub_handle = subscription_handle.clone();
        tokio::spawn(async move {
            while let Some(message) = sub_handle.recv().await {
                let response = match message {
                    StreamingEvent::CoreRawBlock { data } => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            payload_size = data.len(),
                            "block_headers=forward_block"
                        );
                        let block_headers = BlockHeaders {
                            headers: vec![data],
                        };
                        let response = BlockHeadersWithChainLocksResponse {
                            responses: Some(
                                dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(block_headers)
                            ),
                        };

                        Ok(response)
                    }
                    StreamingEvent::CoreChainLock { data } => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            payload_size = data.len(),
                            "block_headers=forward_chain_lock"
                        );
                        let response = BlockHeadersWithChainLocksResponse {
                            responses: Some(
                                dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::ChainLock(data)
                            ),
                        };

                        Ok(response)
                    }
                    _ => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            event = ?message,
                            "block_headers=ignore_event"
                        );
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if tx.send(response).is_err() {
                    debug!(
                        subscriber_id = sub_handle.id(),
                        "block_headers=client_disconnected"
                    );
                    break;
                }
            }
            debug!(
                subscriber_id = sub_handle.id(),
                "block_headers=subscription_task_finished"
            );
        });

        // Handle historical data if requested
        if count > 0 {
            if let Some(from_block) = from_block {
                match from_block {
                    dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHash(hash) => {
                        // TODO: Process historical block headers from block hash
                        debug!(subscriber_id, ?hash, "block_headers=historical_from_hash_request");
                        self.process_historical_blocks_from_hash(&hash, count as usize)
                            .await?;
                    }
                    dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(height) => {
                        // TODO: Process historical block headers from height
                        debug!(subscriber_id, height, "block_headers=historical_from_height_request");
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
        debug!(subscriber_id, "block_headers=stream_ready");
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
        trace!("block_headers=historical_from_hash_unimplemented");
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
        trace!("block_headers=historical_from_height_unimplemented");
        Ok(())
    }
}
