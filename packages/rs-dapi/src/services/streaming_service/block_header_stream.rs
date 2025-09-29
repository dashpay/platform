use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock;
use dapi_grpc::core::v0::{
    BlockHeaders, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::consensus::encode::serialize as serialize_consensus;
use tokio::sync::{Mutex as AsyncMutex, Notify, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, trace, warn};

use crate::DapiError;
use crate::services::streaming_service::{
    FilterType, StreamingEvent, StreamingServiceImpl, SubscriptionHandle,
};

const BLOCK_HEADER_STREAM_BUFFER: usize = 512;

type BlockHeaderResponseResult = Result<BlockHeadersWithChainLocksResponse, Status>;
type BlockHeaderResponseSender = mpsc::Sender<BlockHeaderResponseResult>;
type BlockHeaderResponseStream = ReceiverStream<BlockHeaderResponseResult>;
type BlockHeaderResponse = Response<BlockHeaderResponseStream>;
type DeliveredHashSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveryGate = Arc<AtomicBool>;
type DeliveryNotify = Arc<Notify>;

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

        if let FromBlock::FromBlockHeight(height) = &from_block
            && *height == 0
        {
            warn!(height, "block_headers=invalid_starting_height");
            return Err(Status::invalid_argument(
                "Minimum value for from_block_height is 1",
            ));
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

        self.send_initial_chainlock(tx.clone()).await?;

        self.fetch_historical_blocks(from_block, Some(count as usize), None, tx)
            .await?;

        let stream: BlockHeaderResponseStream = ReceiverStream::new(rx);
        debug!("block_headers=historical_stream_ready");
        Ok(Response::new(stream))
    }

    async fn handle_combined_mode(
        &self,
        from_block: FromBlock,
    ) -> Result<BlockHeaderResponse, Status> {
        let (tx, rx) = mpsc::channel(BLOCK_HEADER_STREAM_BUFFER);
        let delivered_hashes: DeliveredHashSet = Arc::new(AsyncMutex::new(HashSet::new()));
        let delivery_gate: DeliveryGate = Arc::new(AtomicBool::new(false));
        let delivery_notify: DeliveryNotify = Arc::new(Notify::new());

        let subscriber_id = self
            .start_live_stream(
                tx.clone(),
                delivered_hashes.clone(),
                delivery_gate.clone(),
                delivery_notify.clone(),
            )
            .await;
        self.send_initial_chainlock(tx.clone()).await?;
        self.fetch_historical_blocks(from_block, None, Some(delivered_hashes.clone()), tx.clone())
            .await?;
        delivery_gate.store(true, Ordering::Release);
        delivery_notify.notify_waiters();
        let stream: BlockHeaderResponseStream = ReceiverStream::new(rx);
        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=stream_ready"
        );
        Ok(Response::new(stream))
    }

    async fn start_live_stream(
        &self,
        tx: BlockHeaderResponseSender,
        delivered_hashes: DeliveredHashSet,
        delivery_gate: DeliveryGate,
        delivery_notify: DeliveryNotify,
    ) -> String {
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

        self.workers.spawn(async move {
            Self::block_header_worker(
                block_handle,
                chainlock_handle,
                tx,
                delivered_hashes,
                delivery_gate,
                delivery_notify,
            )
            .await;
            Ok::<(), DapiError>(())
        });

        subscriber_id
    }

    async fn send_initial_chainlock(&self, tx: BlockHeaderResponseSender) -> Result<(), Status> {
        if let Some(chain_lock) = self
            .core_client
            .get_best_chain_lock()
            .await
            .map_err(Status::from)?
        {
            trace!(?chain_lock, "block_headers=initial_chain_lock");
            let chain_lock_bytes = serialize_consensus(&chain_lock);
            let response = BlockHeadersWithChainLocksResponse {
                responses: Some(
                    dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::ChainLock(
                        chain_lock_bytes,
                    ),
                ),
            };
            // Failure means client is already gone; treat as success.
            let _ = tx.send(Ok(response)).await;
        }
        Ok(())
    }

    async fn block_header_worker(
        block_handle: SubscriptionHandle,
        chainlock_handle: SubscriptionHandle,
        tx: BlockHeaderResponseSender,
        delivered_hashes: DeliveredHashSet,
        delivery_gate: DeliveryGate,
        delivery_notify: DeliveryNotify,
    ) {
        let subscriber_id = block_handle.id().to_string();
        let mut pending: Vec<StreamingEvent> = Vec::new();
        let mut gated = !delivery_gate.load(Ordering::Acquire);

        loop {
            tokio::select! {
                _ = delivery_notify.notified(), if gated => {
                    gated = !delivery_gate.load(Ordering::Acquire);
                    if !gated
                        && !Self::flush_pending(&subscriber_id, &tx, &delivered_hashes, &mut pending).await {
                            break;
                        }
                }
                message = block_handle.recv() => {
                    match message {
                        Some(event) => {
                            if gated {
                                pending.push(event);
                                continue;
                            }
                            if !Self::forward_event(event, &subscriber_id, &tx, &delivered_hashes).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                message = chainlock_handle.recv() => {
                    match message {
                        Some(event) => {
                            if gated {
                                pending.push(event);
                                continue;
                            }
                            if !Self::forward_event(event, &subscriber_id, &tx, &delivered_hashes).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=subscription_task_finished"
        );
    }

    async fn flush_pending(
        subscriber_id: &str,
        tx: &BlockHeaderResponseSender,
        delivered_hashes: &DeliveredHashSet,
        pending: &mut Vec<StreamingEvent>,
    ) -> bool {
        if pending.is_empty() {
            return true;
        }

        let queued: Vec<StreamingEvent> = std::mem::take(pending);
        for event in queued {
            if !Self::forward_event(event, subscriber_id, tx, delivered_hashes).await {
                return false;
            }
        }
        true
    }

    async fn forward_event(
        event: StreamingEvent,
        subscriber_id: &str,
        tx: &BlockHeaderResponseSender,
        delivered_hashes: &DeliveredHashSet,
    ) -> bool {
        let maybe_response = match event {
            StreamingEvent::CoreRawBlock { data } => {
                let block_hash_hex = Self::block_hash_hex_from_block_bytes(&data)
                    .unwrap_or_else(|| "n/a".to_string());
                let mut allow_forward = true;
                if block_hash_hex != "n/a"
                    && let Ok(hash_bytes) = hex::decode(&block_hash_hex)
                {
                    let mut hashes = delivered_hashes.lock().await;
                    if hashes.remove(&hash_bytes) {
                        trace!(
                            subscriber_id,
                            block_hash = %block_hash_hex,
                            "block_headers=skip_duplicate_block"
                        );
                        allow_forward = false;
                    } else {
                        hashes.insert(hash_bytes);
                    }
                }

                if !allow_forward {
                    return true;
                }

                trace!(
                    subscriber_id,
                    block_hash = %block_hash_hex,
                    payload_size = data.len(),
                    "block_headers=forward_block"
                );
                let block_headers = BlockHeaders {
                    headers: vec![data],
                };
                Some(Ok(BlockHeadersWithChainLocksResponse {
                    responses: Some(
                        dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(block_headers),
                    ),
                }))
            }
            StreamingEvent::CoreChainLock { data } => {
                trace!(
                    subscriber_id,
                    payload_size = data.len(),
                    "block_headers=forward_chain_lock"
                );
                Some(Ok(BlockHeadersWithChainLocksResponse {
                    responses: Some(
                        dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::ChainLock(data),
                    ),
                }))
            }
            other => {
                let summary = Self::summarize_streaming_event(&other);
                trace!(
                    subscriber_id,
                    event = %summary,
                    "block_headers=ignore_event"
                );
                None
            }
        };

        if let Some(response) = maybe_response
            && tx.send(response).await.is_err()
        {
            debug!(subscriber_id, "block_headers=client_disconnected");
            return false;
        }
        true
    }

    async fn fetch_historical_blocks(
        &self,
        from_block: FromBlock,
        limit: Option<usize>,
        delivered_hashes: Option<DeliveredHashSet>,
        tx: BlockHeaderResponseSender,
    ) -> Result<(), Status> {
        use std::str::FromStr;

        let (start_height, count_target) = match from_block {
            FromBlock::FromBlockHash(hash) => {
                let hash_hex = hex::encode(&hash);
                let block_hash = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
                    .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;
                let header = self
                    .core_client
                    .get_block_header_info(&block_hash)
                    .await
                    .map_err(Status::from)?;
                let start = header.height as usize;
                let desired = if let Some(limit) = limit {
                    limit
                } else {
                    let best = self
                        .core_client
                        .get_block_count()
                        .await
                        .map_err(Status::from)? as usize;
                    best.saturating_sub(start).saturating_add(1)
                };
                debug!(start, desired, "block_headers=historical_from_hash_request");
                (start, desired)
            }
            FromBlock::FromBlockHeight(height) => {
                let start = height as usize;
                let desired = if let Some(limit) = limit {
                    limit
                } else {
                    let best = self
                        .core_client
                        .get_block_count()
                        .await
                        .map_err(Status::from)? as usize;
                    best.saturating_sub(start).saturating_add(1)
                };
                debug!(
                    start,
                    desired, "block_headers=historical_from_height_request"
                );
                (start, desired)
            }
        };

        if count_target == 0 {
            return Ok(());
        }

        // Align with historical JS behaviour: count cannot exceed tip.
        let best_height = self
            .core_client
            .get_block_count()
            .await
            .map_err(Status::from)? as usize;
        if start_height >= best_height.saturating_add(1) {
            warn!(start_height, best_height, "block_headers=start_beyond_tip");
            return Err(Status::not_found(format!(
                "Block {} not found",
                start_height
            )));
        }
        let max_available = best_height.saturating_sub(start_height).saturating_add(1);
        if count_target > max_available {
            warn!(
                start_height,
                requested = count_target,
                max_available,
                "block_headers=count_exceeds_tip"
            );
            return Err(Status::invalid_argument("count exceeds chain tip"));
        }

        self.process_historical_blocks_from_height(start_height, count_target, delivered_hashes, tx)
            .await
    }

    /// Process historical blocks from a specific block height
    async fn process_historical_blocks_from_height(
        &self,
        from_height: usize,
        count: usize,
        delivered_hashes: Option<DeliveredHashSet>,
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

            let hash_bytes =
                <dashcore_rpc::dashcore::BlockHash as AsRef<[u8]>>::as_ref(&hash).to_vec();

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

            if let Some(ref shared) = delivered_hashes {
                let mut hashes = shared.lock().await;
                hashes.insert(hash_bytes);
            }

            while collected.len() >= CHUNK_SIZE {
                let bh = BlockHeaders {
                    headers: collected.drain(..CHUNK_SIZE).collect(),
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

            // CoreClient handles RPC flow control, so no additional pacing is required here.
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
