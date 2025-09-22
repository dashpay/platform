use dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock;
use dapi_grpc::core::v0::{
    BlockHeaders, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, trace, warn};

use crate::services::streaming_service::{
    FilterType, StreamingEvent, StreamingServiceImpl, SubscriptionHandle,
};

const BLOCK_HEADER_STREAM_BUFFER: usize = 512;

type BlockHeaderResponseResult = Result<BlockHeadersWithChainLocksResponse, Status>;
type BlockHeaderResponseSender = mpsc::Sender<BlockHeaderResponseResult>;
type BlockHeaderResponseStream = ReceiverStream<BlockHeaderResponseResult>;
type BlockHeaderResponse = Response<BlockHeaderResponseStream>;

impl StreamingServiceImpl {
    pub async fn subscribe_to_block_headers_with_chain_locks_impl(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<BlockHeaderResponse, Status> {
        trace!("subscribe_to_block_headers_with_chain_locks_impl=begin");
        let req = request.into_inner();

        // Validate parameters
        let count = req.count;
        let from_block = match req.from_block {
            Some(from_block) => from_block,
            None => {
                warn!("block_headers=missing_from_block");
                return Err(Status::invalid_argument("Must specify from_block"));
            }
        };

        trace!(count, "block_headers=request_parsed");

        if let FromBlock::FromBlockHeight(height) = &from_block {
            if *height == 0 {
                warn!(height, "block_headers=invalid_starting_height");
                return Err(Status::invalid_argument(
                    "Minimum value for from_block_height is 1",
                ));
            }
        }

        let response = if count > 0 {
            self.handle_historical_mode(from_block, count).await?
        } else {
            self.handle_combined_mode(from_block).await?
        };

        Ok(response)
    }

    async fn handle_historical_mode(
        &self,
        from_block: FromBlock,
        count: u32,
    ) -> Result<BlockHeaderResponse, Status> {
        let (tx, rx) = mpsc::channel(BLOCK_HEADER_STREAM_BUFFER);

        match from_block {
            FromBlock::FromBlockHash(hash) => {
                debug!(
                    hash = %hex::encode(&hash),
                    count,
                    "block_headers=historical_from_hash_request"
                );
                self.process_historical_blocks_from_hash(&hash, count as usize, tx)
                    .await?;
            }
            FromBlock::FromBlockHeight(height) => {
                debug!(
                    height,
                    count, "block_headers=historical_from_height_request"
                );
                self.process_historical_blocks_from_height(height as usize, count as usize, tx)
                    .await?;
            }
        }

        let stream: BlockHeaderResponseStream = ReceiverStream::new(rx);
        debug!("block_headers=historical_stream_ready");
        Ok(Response::new(stream))
    }

    async fn handle_combined_mode(
        &self,
        from_block: FromBlock,
    ) -> Result<BlockHeaderResponse, Status> {
        let (tx, rx) = mpsc::channel(BLOCK_HEADER_STREAM_BUFFER);
        let subscriber_id = self.start_live_stream(tx.clone()).await;
        self.backfill_to_tip(from_block, tx).await?;
        let stream: BlockHeaderResponseStream = ReceiverStream::new(rx);
        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=stream_ready"
        );
        Ok(Response::new(stream))
    }

    async fn start_live_stream(&self, tx: BlockHeaderResponseSender) -> String {
        let filter = FilterType::CoreAllBlocks;
        let block_handle = self.subscriber_manager.add_subscription(filter).await;
        let subscriber_id = block_handle.id().to_string();
        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=subscription_created"
        );

        let chainlock_handle = self
            .subscriber_manager
            .add_subscription(FilterType::CoreChainLocks)
            .await;
        debug!(
            subscriber_id = chainlock_handle.id(),
            "block_headers=chainlock_subscription_created"
        );

        Self::spawn_block_header_worker(block_handle, chainlock_handle, tx);

        subscriber_id
    }

    fn spawn_block_header_worker(
        block_handle: SubscriptionHandle,
        chainlock_handle: SubscriptionHandle,
        tx: BlockHeaderResponseSender,
    ) {
        tokio::spawn(async move {
            Self::block_header_worker(block_handle, chainlock_handle, tx).await;
        });
    }

    async fn block_header_worker(
        block_handle: SubscriptionHandle,
        chainlock_handle: SubscriptionHandle,
        tx: BlockHeaderResponseSender,
    ) {
        let subscriber_id = block_handle.id().to_string();

        while let Some(message) = tokio::select! {
            m = block_handle.recv() => m,
            m = chainlock_handle.recv() => m,
        } {
            let response = match message {
                StreamingEvent::CoreRawBlock { data } => {
                    let block_hash = Self::block_hash_hex_from_block_bytes(&data)
                        .unwrap_or_else(|| "n/a".to_string());
                    trace!(
                        subscriber_id = subscriber_id.as_str(),
                        block_hash = %block_hash,
                        payload_size = data.len(),
                        "block_headers=forward_block"
                    );
                    let block_headers = BlockHeaders {
                        headers: vec![data],
                    };
                    let response = BlockHeadersWithChainLocksResponse {
                        responses: Some(
                            dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(block_headers),
                        ),
                    };
                    Ok(response)
                }
                StreamingEvent::CoreChainLock { data } => {
                    trace!(
                        subscriber_id = subscriber_id.as_str(),
                        payload_size = data.len(),
                        "block_headers=forward_chain_lock"
                    );
                    let response = BlockHeadersWithChainLocksResponse {
                        responses: Some(
                            dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::ChainLock(data),
                        ),
                    };
                    Ok(response)
                }
                other => {
                    let summary = Self::summarize_streaming_event(&other);
                    trace!(
                        subscriber_id = subscriber_id.as_str(),
                        event = %summary,
                        "block_headers=ignore_event"
                    );
                    continue;
                }
            };

            if tx.send(response).await.is_err() {
                debug!(
                    subscriber_id = subscriber_id.as_str(),
                    "block_headers=client_disconnected"
                );
                break;
            }
        }

        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=subscription_task_finished"
        );
    }

    async fn backfill_to_tip(
        &self,
        from_block: FromBlock,
        tx: BlockHeaderResponseSender,
    ) -> Result<(), Status> {
        // Snapshot best height first to guarantee no gaps between backfill and live stream
        let best = self
            .core_client
            .get_block_count()
            .await
            .map_err(Status::from)? as usize;

        match from_block {
            FromBlock::FromBlockHash(hash) => {
                use std::str::FromStr;
                let hash_hex = hex::encode(&hash);
                let block_hash = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
                    .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;
                let header = self
                    .core_client
                    .get_block_header_info(&block_hash)
                    .await
                    .map_err(Status::from)?;
                if header.height > 0 {
                    let start = header.height as usize;
                    let count_tip = best.saturating_sub(start).saturating_add(1);
                    debug!(start, count_tip, "block_headers=backfill_from_hash");
                    self.process_historical_blocks_from_height(start, count_tip, tx.clone())
                        .await?;
                }
            }
            FromBlock::FromBlockHeight(height) => {
                let start = height as usize;
                if start >= 1 {
                    let count_tip = best.saturating_sub(start).saturating_add(1);
                    debug!(start, count_tip, "block_headers=backfill_from_height");
                    self.process_historical_blocks_from_height(start, count_tip, tx.clone())
                        .await?;
                }
            }
        }

        Ok(())
    }

    /// Process historical blocks from a specific block hash
    async fn process_historical_blocks_from_hash(
        &self,
        from_hash: &[u8],
        count: usize,
        tx: BlockHeaderResponseSender,
    ) -> Result<(), Status> {
        use std::str::FromStr;
        // Derive starting height from hash, then delegate to height-based fetch
        let hash_hex = hex::encode(from_hash);
        let hash = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
            .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;

        let header_info = self
            .core_client
            .get_block_header_info(&hash)
            .await
            .map_err(Status::from)?;

        let start_height = header_info.height as usize;
        self.process_historical_blocks_from_height(start_height, count, tx)
            .await
    }

    /// Process historical blocks from a specific block height
    async fn process_historical_blocks_from_height(
        &self,
        from_height: usize,
        count: usize,
        tx: BlockHeaderResponseSender,
    ) -> Result<(), Status> {
        // Fetch blocks sequentially and send only block headers (80 bytes each)
        // Chunk responses to avoid huge gRPC messages.
        const CHUNK_SIZE: usize = 1000;

        trace!(
            from_height,
            count, "block_headers=historical_from_height_begin"
        );

        let mut collected: Vec<Vec<u8>> = Vec::with_capacity(CHUNK_SIZE);
        let mut sent: usize = 0;

        for i in 0..count {
            let height = (from_height + i) as u32;
            // Resolve hash
            let hash = match self.core_client.get_block_hash(height).await {
                Ok(h) => h,
                Err(e) => {
                    // Stop on first error (e.g., height beyond tip)
                    trace!(height, error = ?e, "block_headers=historical_get_block_hash_failed");
                    break;
                }
            };

            // Fetch block bytes and slice header (first 80 bytes)
            let block_bytes = match self.core_client.get_block_bytes_by_hash(hash).await {
                Ok(b) => b,
                Err(e) => {
                    trace!(height, error = ?e, "block_headers=historical_get_block_failed");
                    break;
                }
            };
            if block_bytes.len() < 80 {
                // Malformed block; abort
                return Err(Status::internal(
                    "Received malformed block bytes (len < 80)",
                ));
            }
            let header_bytes = block_bytes[..80].to_vec();
            collected.push(header_bytes);

            if collected.len() >= CHUNK_SIZE {
                let bh = BlockHeaders {
                    headers: collected.drain(..).collect(),
                };
                let response = BlockHeadersWithChainLocksResponse {
                    responses: Some(
                        dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(bh),
                    ),
                };
                if tx.send(Ok(response)).await.is_err() {
                    debug!("block_headers=historical_client_disconnected");
                    return Ok(());
                }
                sent += CHUNK_SIZE;
            }
        }

        // Flush remaining headers
        if !collected.is_empty() {
            let bh = BlockHeaders { headers: collected };
            let response = BlockHeadersWithChainLocksResponse {
                responses: Some(
                    dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(bh),
                ),
            };
            if tx.send(Ok(response)).await.is_err() {
                debug!("block_headers=historical_client_disconnected");
                return Ok(());
            }
            sent += 1; // mark as sent (approximate)
        }

        trace!(
            from_height,
            count, sent, "block_headers=historical_from_height_end"
        );
        Ok(())
    }
}
