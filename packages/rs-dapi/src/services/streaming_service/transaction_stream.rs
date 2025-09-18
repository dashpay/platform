use dapi_grpc::core::v0::{
    InstantSendLockMessages, RawTransactions, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info, trace, warn};

use crate::services::streaming_service::StreamingServiceImpl;
use crate::services::streaming_service::bloom::bloom_flags_from_int;
use crate::services::streaming_service::subscriber_manager::{FilterType, StreamingEvent};

impl StreamingServiceImpl {
    pub async fn subscribe_to_transactions_with_proofs_impl(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<
        Response<UnboundedReceiverStream<Result<TransactionsWithProofsResponse, Status>>>,
        Status,
    > {
        trace!("transactions_with_proofs=subscribe_begin");
        let req = request.into_inner();

        // Extract bloom filter parameters
        let bloom_filter = req
            .bloom_filter
            .ok_or_else(|| Status::invalid_argument("bloom_filter is required"))?;

        trace!(
            n_hash_funcs = bloom_filter.n_hash_funcs,
            n_tweak = bloom_filter.n_tweak,
            v_data_len = bloom_filter.v_data.len(),
            count = req.count,
            has_from_block = req.from_block.is_some(),
            "transactions_with_proofs=request_parsed"
        );

        // Validate bloom filter parameters
        if bloom_filter.v_data.is_empty() {
            warn!("transactions_with_proofs=bloom_filter_empty");
            return Err(Status::invalid_argument(
                "bloom filter data cannot be empty",
            ));
        }

        if bloom_filter.n_hash_funcs == 0 {
            warn!("transactions_with_proofs=bloom_filter_no_hash_funcs");
            return Err(Status::invalid_argument(
                "number of hash functions must be greater than 0",
            ));
        }

        // Create filter from bloom filter parameters
        let bloom_filter_clone = bloom_filter.clone();
        let count = req.count;
        let flags = bloom_flags_from_int(bloom_filter_clone.n_flags);
        let core_filter = dashcore_rpc::dashcore::bloom::BloomFilter::from_bytes(
            bloom_filter_clone.v_data.clone(),
            bloom_filter_clone.n_hash_funcs,
            bloom_filter_clone.n_tweak,
            flags,
        )
        .map_err(|e| Status::invalid_argument(format!("invalid bloom filter data: {}", e)))?;

        let filter = FilterType::CoreBloomFilter(
            std::sync::Arc::new(std::sync::RwLock::new(core_filter)),
            flags,
        );

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // If historical-only requested (count > 0), send historical data and close the stream
        if count > 0 {
            let tx_hist = tx.clone();
            let from_block = req.from_block.ok_or_else(|| {
                Status::invalid_argument("Must specify from_block when count > 0")
            })?;

            match from_block {
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHash(hash) => {
                    debug!(
                        hash = %hex::encode(&hash),
                        count,
                        "transactions_with_proofs=historical_from_hash_request"
                    );
                    self.process_historical_transactions_from_hash(&hash, count as usize, &bloom_filter_clone, tx_hist)
                        .await?;
                }
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(height) => {
                    debug!(height, count, "transactions_with_proofs=historical_from_height_request");
                    self.process_historical_transactions_from_height(
                        height as usize,
                        count as usize,
                        &bloom_filter_clone,
                        tx_hist,
                    )
                    .await?;
                }
            }

            let stream = UnboundedReceiverStream::new(rx);
            debug!("transactions_with_proofs=historical_stream_ready");
            return Ok(Response::new(stream));
        }

        // Add subscription to manager for live updates (subscribe first to avoid races)
        let subscription_handle = self.subscriber_manager.add_subscription(filter).await;
        let subscriber_id = subscription_handle.id().to_string();
        debug!(
            subscriber_id,
            "transactions_with_proofs=subscription_created"
        );

        info!(
            "Started transaction subscription: {}",
            subscription_handle.id()
        );

        // Spawn task to convert internal messages to gRPC responses
        let sub_handle = subscription_handle.clone();
        let tx_live = tx.clone();
        tokio::spawn(async move {
            trace!(
                subscriber_id = sub_handle.id(),
                "transactions_with_proofs=worker_started"
            );
            while let Some(message) = sub_handle.recv().await {
                let response = match message {
                    StreamingEvent::CoreRawTransaction { data: tx_data } => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            payload_size = tx_data.len(),
                            "transactions_with_proofs=forward_raw_transaction"
                        );
                        let raw_transactions = RawTransactions {
                            transactions: vec![tx_data],
                        };

                        let response = TransactionsWithProofsResponse {
                            responses: Some(
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_transactions),
                            ),
                        };

                        Ok(response)
                    }
                    StreamingEvent::CoreRawBlock { data } => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            payload_size = data.len(),
                            "transactions_with_proofs=forward_merkle_block"
                        );
                        let response = TransactionsWithProofsResponse {
                            responses: Some(
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data),
                            ),
                        };

                        Ok(response)
                    }
                    StreamingEvent::CoreInstantLock { data } => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            payload_size = data.len(),
                            "transactions_with_proofs=forward_instant_lock"
                        );
                        let instant_lock_messages = InstantSendLockMessages {
                            messages: vec![data],
                        };

                        let response = TransactionsWithProofsResponse {
                            responses: Some(
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::InstantSendLockMessages(instant_lock_messages),
                            ),
                        };

                        Ok(response)
                    }
                    _ => {
                        trace!(
                            subscriber_id = sub_handle.id(),
                            event = ?message,
                            "transactions_with_proofs=ignore_event"
                        );
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if tx_live.send(response).is_err() {
                    debug!(
                        subscriber_id = sub_handle.id(),
                        "transactions_with_proofs=client_disconnected"
                    );
                    break;
                }
            }
            // Drop of the handle will remove the subscription automatically
            info!(
                subscriber_id = sub_handle.id(),
                "transactions_with_proofs=worker_finished"
            );
        });

        // After subscribing, backfill historical up to the current tip (if requested via from_block)
        if let Some(from_block) = req.from_block.clone() {
            let tx_hist = tx.clone();
            let best = self
                .core_client
                .get_block_count()
                .await
                .map_err(Status::from)? as usize;

            match from_block {
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHash(hash) => {
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
                        let height = hi.height as usize;
                        let count_tip = best.saturating_sub(height).saturating_add(1);
                        debug!(height, count_tip, "transactions_with_proofs=historical_tip_from_hash");
                        self.process_historical_transactions_from_height(
                            height,
                            count_tip,
                            &bloom_filter_clone,
                            tx_hist,
                        )
                        .await?;
                    }
                }
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(height) => {
                    let height = height as usize;
                    if height >= 1 {
                        let count_tip = best.saturating_sub(height).saturating_add(1);
                        debug!(height, count_tip, "transactions_with_proofs=historical_tip_from_height");
                        self.process_historical_transactions_from_height(
                            height,
                            count_tip,
                            &bloom_filter_clone,
                            tx_hist,
                        )
                        .await?;
                    }
                }
            }
        }

        // Process mempool transactions if needed (TODO parity)
        debug!(
            subscriber_id,
            "transactions_with_proofs=streaming_mempool_mode"
        );

        let stream = UnboundedReceiverStream::new(rx);
        debug!(subscriber_id, "transactions_with_proofs=stream_ready");
        Ok(Response::new(stream))
    }

    /// Process historical transactions from a specific block hash
    async fn process_historical_transactions_from_hash(
        &self,
        from_hash: &[u8],
        count: usize,
        bloom_filter: &dapi_grpc::core::v0::BloomFilter,
        tx: mpsc::UnboundedSender<Result<TransactionsWithProofsResponse, Status>>,
    ) -> Result<(), Status> {
        use std::str::FromStr;
        let hash_hex = hex::encode(from_hash);
        let bh = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
            .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;
        let header_info = self
            .core_client
            .get_block_header_info(&bh)
            .await
            .map_err(Status::from)?;
        let start_height = header_info.height as usize;
        self.process_historical_transactions_from_height(start_height, count, bloom_filter, tx)
            .await
    }

    /// Process historical transactions from a specific block height
    async fn process_historical_transactions_from_height(
        &self,
        from_height: usize,
        count: usize,
        bloom_filter: &dapi_grpc::core::v0::BloomFilter,
        tx: mpsc::UnboundedSender<Result<TransactionsWithProofsResponse, Status>>,
    ) -> Result<(), Status> {
        use dashcore_rpc::dashcore::Transaction as CoreTx;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;

        trace!(
            from_height,
            count, "transactions_with_proofs=historical_begin"
        );

        // Clamp to tip
        let tip = self
            .core_client
            .get_block_count()
            .await
            .map_err(Status::from)? as usize;
        if from_height == 0 {
            return Err(Status::invalid_argument(
                "Minimum value for `fromBlockHeight` is 1",
            ));
        }
        if from_height > tip.saturating_add(1) {
            return Err(Status::not_found(format!(
                "Block height {} out of range (tip={})",
                from_height, tip
            )));
        }

        let max_count = tip.saturating_sub(from_height).saturating_add(1);
        let effective = count.min(max_count);

        // Reconstruct bloom filter to perform matching
        let flags = bloom_flags_from_int(bloom_filter.n_flags);
        let mut core_filter = dashcore_rpc::dashcore::bloom::BloomFilter::from_bytes(
            bloom_filter.v_data.clone(),
            bloom_filter.n_hash_funcs,
            bloom_filter.n_tweak,
            flags,
        )
        .map_err(|e| Status::invalid_argument(format!("invalid bloom filter data: {}", e)))?;

        for i in 0..effective {
            let height = (from_height + i) as u32;
            // Resolve hash and fetch block bytes
            let hash = match self.core_client.get_block_hash(height).await {
                Ok(h) => h,
                Err(e) => {
                    trace!(height, error = ?e, "transactions_with_proofs=get_block_hash_failed");
                    break;
                }
            };
            // Fetch raw block bytes and transaction bytes list (without parsing whole block)
            let block_bytes = match self.core_client.get_block_bytes_by_hash(hash).await {
                Ok(b) => b,
                Err(e) => {
                    trace!(height, error = ?e, "transactions_with_proofs=get_block_raw_with_txs_failed");
                    break;
                }
            };
            let txs_bytes = match self
                .core_client
                .get_block_transactions_bytes_by_hash(hash)
                .await
            {
                Ok(t) => t,
                Err(e) => {
                    warn!(height, error = ?e, "transactions_with_proofs=get_block_txs_failed, skipping block");
                    continue;
                }
            };

            let mut matching: Vec<Vec<u8>> = Vec::new();
            for tx_bytes in txs_bytes.iter() {
                // Try to parse each transaction individually; skip if parsing fails
                match deserialize::<CoreTx>(tx_bytes.as_slice()) {
                    Ok(tx) => {
                        if super::bloom::matches_transaction(&mut core_filter, &tx, flags) {
                            // If matched, forward original bytes
                            matching.push(tx_bytes.clone());
                        }
                    }
                    Err(e) => {
                        tracing::debug!(height, error = %e, "Failed to deserialize transaction; skipping for bloom match");
                        continue;
                    }
                }
            }

            // First, send transactions (if any)
            if !matching.is_empty() {
                let raw_transactions = RawTransactions {
                    transactions: matching,
                };
                let response = TransactionsWithProofsResponse {
                    responses: Some(
                        dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_transactions),
                    ),
                };
                if tx.send(Ok(response)).is_err() {
                    debug!("transactions_with_proofs=historical_client_disconnected");
                    return Ok(());
                }
            }

            // Then, send merkle block placeholder (raw block) to indicate block boundary
            let response = TransactionsWithProofsResponse {
                responses: Some(
                    dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(block_bytes),
                ),
            };
            if tx.send(Ok(response)).is_err() {
                debug!("transactions_with_proofs=historical_client_disconnected");
                return Ok(());
            }

            // Pace requests slightly to avoid Core overload
            // sleep(Duration::from_millis(1)).await;
        }

        trace!(
            from_height,
            effective, "transactions_with_proofs=historical_end"
        );
        Ok(())
    }
}
