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

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // If count > 0, this is a historical-only stream.
        // We must send the requested headers and then end the stream (no live updates).
        if count > 0 {
            match from_block {
                Some(dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHash(hash)) => {
                    debug!(
                        hash = %hex::encode(&hash),
                        count,
                        "block_headers=historical_from_hash_request"
                    );
                    self.process_historical_blocks_from_hash(&hash, count as usize, tx)
                        .await?;
                }
                Some(dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(height)) => {
                    debug!(height, count, "block_headers=historical_from_height_request");
                    self.process_historical_blocks_from_height(height as usize, count as usize, tx)
                        .await?;
                }
                None => unreachable!(),
            }

            let stream = UnboundedReceiverStream::new(rx);
            debug!("block_headers=historical_stream_ready");
            return Ok(Response::new(stream));
        }

        // Otherwise (count == 0), subscribe for continuous updates.
        // Create filter (no filtering needed for block headers - all blocks)
        let filter = FilterType::CoreAllBlocks;

        // Add subscription to manager
        let sub_handle = self.subscriber_manager.add_subscription(filter).await;
        let subscriber_id = sub_handle.id().to_string();
        debug!(subscriber_id, "block_headers=subscription_created");

        let chainlock_handle = self
            .subscriber_manager
            .add_subscription(FilterType::CoreChainLocks)
            .await;
        debug!(
            subscriber_id = chainlock_handle.id(),
            "block_headers=chainlock_subscription_created"
        );

        // Spawn task to convert internal messages to gRPC responses
        let tx_live = tx.clone();
        tokio::spawn(async move {
            while let Some(message) = tokio::select! {
                m = sub_handle.recv() => m,
                m = chainlock_handle.recv() => m,
            } {
                let response = match message {
                    StreamingEvent::CoreRawBlock { data } => {
                        let block_hash = super::StreamingServiceImpl::block_hash_hex_from_block_bytes(&data)
                            .unwrap_or_else(|| "n/a".to_string());
                        trace!(
                            subscriber_id = sub_handle.id(),
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
                            subscriber_id = sub_handle.id(),
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
                    _ => {
                        let summary = super::StreamingServiceImpl::summarize_streaming_event(&message);
                        trace!(
                            subscriber_id = sub_handle.id(),
                            event = %summary,
                            "block_headers=ignore_event"
                        );
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if tx_live.send(response).is_err() {
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

        // After subscribing, optionally backfill historical headers to the current tip
        if let Some(from_block) = req.from_block {
            // Snapshot best height first to guarantee no gaps between backfill and live stream
            let best = self
                .core_client
                .get_block_count()
                .await
                .map_err(Status::from)? as usize;

            match from_block {
                dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHash(hash) => {
                    use std::str::FromStr;
                    let hash_hex = hex::encode(&hash);
                    let bh = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
                        .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;
                    let hi = self
                        .core_client
                        .get_block_header_info(&bh)
                        .await
                        .map_err(Status::from)?;
                    if hi.height > 0 {
                        let start = hi.height as usize;
                        let count_tip = best.saturating_sub(start).saturating_add(1);
                        debug!(start, count_tip, "block_headers=backfill_from_hash");
                        self
                            .process_historical_blocks_from_height(start, count_tip, tx.clone())
                            .await?;
                    }
                }
                dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock::FromBlockHeight(height) => {
                    let start = height as usize;
                    if start >= 1 {
                        let count_tip = best.saturating_sub(start).saturating_add(1);
                        debug!(start, count_tip, "block_headers=backfill_from_height");
                        self
                            .process_historical_blocks_from_height(start, count_tip, tx.clone())
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
        from_hash: &[u8],
        count: usize,
        tx: mpsc::UnboundedSender<Result<BlockHeadersWithChainLocksResponse, Status>>,
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
        tx: mpsc::UnboundedSender<Result<BlockHeadersWithChainLocksResponse, Status>>,
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
                if tx.send(Ok(response)).is_err() {
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
            let _ = tx.send(Ok(response));
            sent += 1; // mark as sent (approximate)
        }

        trace!(
            from_height,
            count, sent, "block_headers=historical_from_height_end"
        );
        Ok(())
    }
}
