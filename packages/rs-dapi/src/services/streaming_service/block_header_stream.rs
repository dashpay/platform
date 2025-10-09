use std::collections::HashSet;
use std::sync::Arc;

use dapi_grpc::core::v0::block_headers_with_chain_locks_request::FromBlock;
use dapi_grpc::core::v0::{
    BlockHeaders, BlockHeadersWithChainLocksRequest, BlockHeadersWithChainLocksResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::consensus::encode::{
    deserialize as deserialize_consensus, serialize as serialize_consensus,
};
use dashcore_rpc::dashcore::hashes::Hash;
use tokio::sync::{Mutex as AsyncMutex, mpsc, watch};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, trace};

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
type DeliveryGateSender = watch::Sender<bool>;
type DeliveryGateReceiver = watch::Receiver<bool>;

const MAX_HEADERS_PER_BATCH: usize = 500;
impl StreamingServiceImpl {
    pub async fn subscribe_to_block_headers_with_chain_locks_impl(
        &self,
        request: Request<BlockHeadersWithChainLocksRequest>,
    ) -> Result<BlockHeaderResponse, Status> {
        trace!("subscribe_to_block_headers_with_chain_locks_impl=begin");
        let req = request.into_inner();

        // Validate parameters
        let count = req.count;
        let validation_error = "Minimum value for `fromBlockHeight` is 1";

        let from_block = match req.from_block {
            Some(FromBlock::FromBlockHeight(height)) => {
                if height == 0 {
                    debug!(height, "block_headers=invalid_starting_height");
                    return Err(Status::invalid_argument(validation_error));
                }
                FromBlock::FromBlockHeight(height)
            }
            Some(FromBlock::FromBlockHash(ref hash)) if hash.is_empty() => {
                debug!("block_headers=empty_from_block_hash");
                return Err(Status::invalid_argument(validation_error));
            }
            Some(from_block) => from_block,
            None => {
                debug!("block_headers=missing_from_block");
                return Err(Status::invalid_argument(validation_error));
            }
        };

        trace!(count, "block_headers=request_parsed");

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

        self.spawn_fetch_historical_headers(from_block, Some(count as usize), None, tx, None, None)
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
        let (delivery_gate_tx, delivery_gate_rx) = watch::channel(false);

        let subscriber_id = self
            .start_live_stream(
                tx.clone(),
                delivered_hashes.clone(),
                delivery_gate_rx.clone(),
            )
            .await;
        self.send_initial_chainlock(tx.clone()).await?;
        self.spawn_fetch_historical_headers(
            from_block,
            None,
            Some(delivered_hashes),
            tx,
            Some(delivery_gate_tx),
            Some(subscriber_id.clone()),
        )
        .await?;
        let stream: BlockHeaderResponseStream = ReceiverStream::new(rx);
        debug!(
            subscriber_id = subscriber_id.as_str(),
            "block_headers=stream_ready"
        );
        Ok(Response::new(stream))
    }

    async fn spawn_fetch_historical_headers(
        &self,
        from_block: FromBlock,
        limit: Option<usize>,
        delivered_hashes: Option<DeliveredHashSet>,
        tx: BlockHeaderResponseSender,
        gate: Option<DeliveryGateSender>,
        subscriber_id: Option<String>,
    ) -> Result<(), Status> {
        let service = self.clone();

        self.workers.spawn(async move {
            let result = service
                .fetch_historical_blocks(
                    from_block,
                    limit,
                    delivered_hashes,
                    tx.clone(),
                )
                .await;

            if let Some(gate) = gate {
                let _ = gate.send(true);
            }
            // watch receivers wake via the send above; no separate notification needed.

            match result {
                Ok(()) => {
                    if let Some(ref id) = subscriber_id {
                        debug!(subscriber_id = id.as_str(), "block_headers=historical_fetch_completed");
                    } else {
                        debug!("block_headers=historical_fetch_completed");
                    }
                    Ok(())
                }
                Err(status) => {
                    if let Some(ref id) = subscriber_id {
                        debug!(subscriber_id = id.as_str(), error = %status, "block_headers=historical_fetch_failed");
                    } else {
                        debug!(error = %status, "block_headers=historical_fetch_failed");
                    }
                    let _ = tx.send(Err(status.clone())).await;
                    Err(DapiError::from(status))
                }
            }
        });

        Ok(())
    }

    async fn start_live_stream(
        &self,
        tx: BlockHeaderResponseSender,
        delivered_hashes: DeliveredHashSet,
        delivery_gate: DeliveryGateReceiver,
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
            trace!(
                height = chain_lock.block_height,
                block_hash = %chain_lock.block_hash,
                "block_headers=initial_chain_lock"
            );
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
        mut delivery_gate: DeliveryGateReceiver,
    ) {
        let subscriber_id = block_handle.id().to_string();
        let mut pending: Vec<StreamingEvent> = Vec::new();
        let mut gated = !*delivery_gate.borrow();

        loop {
            tokio::select! {
                gate_change = delivery_gate.changed(), if gated => {
                    if gate_change.is_err() {
                        break;
                    }
                    gated = !*delivery_gate.borrow();
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
                } else {
                    debug!(
                        subscriber_id,
                        block_hash = %block_hash_hex,
                        "block_headers=forward_block_invalid_hash"
                    );
                }

                if !allow_forward {
                    return true;
                }

                if data.len() < 80 {
                    debug!(
                        subscriber_id,
                        payload_size = data.len(),
                        "block_headers=forward_block_short_payload"
                    );
                    return true;
                }

                let header_bytes = data[..80].to_vec();
                trace!(
                    subscriber_id,
                    block_hash = %block_hash_hex,
                    payload_size = data.len(),
                    "block_headers=forward_block"
                );
                let block_headers = BlockHeaders {
                    headers: vec![header_bytes],
                };
                Some(Ok(BlockHeadersWithChainLocksResponse {
                    responses: Some(
                        dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(block_headers),
                    ),
                }))
            }
            StreamingEvent::CoreChainLock { data } => {
                if tracing::enabled!(tracing::Level::TRACE) {
                    if let Ok(chain_lock) =
                        deserialize_consensus::<dashcore_rpc::dashcore::ChainLock>(&data)
                    {
                        trace!(
                            subscriber_id,
                            height = chain_lock.block_height,
                            block_hash = %chain_lock.block_hash,
                            payload_size = data.len(),
                            "block_headers=forward_chain_lock"
                        );
                    } else {
                        trace!(
                            subscriber_id,
                            payload_size = data.len(),
                            "block_headers=forward_chain_lock"
                        );
                    }
                }
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

        if let Some(response) = maybe_response.clone()
            && tx.send(response).await.is_err()
        {
            debug!(subscriber_id, "block_headers=client_disconnected");
            return false;
        }

        trace!(
            subscriber_id,
            response=?maybe_response, "block_headers=event_forwarded"
        );

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

        let best_height = self
            .core_client
            .get_block_count()
            .await
            .map_err(Status::from)? as usize;

        let (start_height, available, desired) = match from_block {
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
                let available = best_height
                    .checked_sub(start)
                    .and_then(|diff| diff.checked_add(1))
                    .unwrap_or(0);
                let desired = limit.unwrap_or(available);
                debug!(start, desired, "block_headers=historical_from_hash_request");
                (start, available, desired)
            }
            FromBlock::FromBlockHeight(height) => {
                let start = height as usize;
                if start == 0 {
                    return Err(Status::invalid_argument(
                        "Minimum value for `fromBlockHeight` is 1",
                    ));
                }
                let available = best_height
                    .checked_sub(start)
                    .and_then(|diff| diff.checked_add(1))
                    .unwrap_or(0);
                let desired = limit.unwrap_or(available);
                debug!(
                    start,
                    desired, "block_headers=historical_from_height_request"
                );
                (start, available, desired)
            }
        };

        if available == 0 {
            return Ok(());
        }

        if start_height >= best_height.saturating_add(1) {
            debug!(start_height, best_height, "block_headers=start_beyond_tip");
            return Err(Status::not_found(format!(
                "Block {} not found",
                start_height
            )));
        }

        if desired == 0 {
            return Ok(());
        }

        if desired > available {
            debug!(
                start_height,
                requested = desired,
                max_available = available,
                "block_headers=count_exceeds_tip"
            );
            return Err(Status::invalid_argument("count exceeds chain tip"));
        }

        let mut remaining = desired;
        let mut current_height = start_height;

        while remaining > 0 {
            let batch_size = remaining.min(MAX_HEADERS_PER_BATCH);

            let mut response_headers = Vec::with_capacity(batch_size);
            let mut hashes_to_store: Vec<Vec<u8>> = Vec::with_capacity(batch_size);

            for offset in 0..batch_size {
                let height = (current_height + offset) as u32;
                let hash = self
                    .core_client
                    .get_block_hash(height)
                    .await
                    .map_err(Status::from)?;
                trace!(
                    height,
                    block_hash = %hash,
                    "block_headers=historical_header_fetched"
                );

                let header_bytes = self
                    .core_client
                    .get_block_header_bytes_by_hash(hash)
                    .await
                    .map_err(Status::from)?;

                if header_bytes.len() < 80 {
                    return Err(Status::internal(
                        "Received malformed block header (len < 80)",
                    ));
                }

                response_headers.push(header_bytes[..80].to_vec());

                if delivered_hashes.is_some() {
                    hashes_to_store.push(hash.to_byte_array().to_vec());
                }
            }

            if let Some(ref shared) = delivered_hashes {
                let mut hashes = shared.lock().await;
                for hash in hashes_to_store {
                    trace!(
                        block_hash = %hex::encode(&hash),
                        "block_headers=delivered_hash_recorded"
                    );
                    hashes.insert(hash);
                }
            }

            let response = BlockHeadersWithChainLocksResponse {
                responses: Some(
                    dapi_grpc::core::v0::block_headers_with_chain_locks_response::Responses::BlockHeaders(
                        BlockHeaders {
                            headers: response_headers,
                        },
                    ),
                ),
            };

            if tx.send(Ok(response)).await.is_err() {
                debug!("block_headers=historical_client_disconnected");
                return Ok(());
            }

            trace!(
                current_height,
                batch_size, remaining, "block_headers=historical_batch_sent"
            );

            remaining = remaining.saturating_sub(batch_size);
            current_height += batch_size;
        }

        Ok(())
    }
}
