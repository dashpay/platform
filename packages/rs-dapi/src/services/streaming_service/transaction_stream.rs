use dapi_grpc::core::v0::{
    InstantSendLockMessages, RawTransactions, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::Block;
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
        let count = req.count;

        let filter = match req.bloom_filter {
            Some(bloom_filter) => {
                let (core_filter, flags) = parse_bloom_filter(&bloom_filter)?;

                FilterType::CoreBloomFilter(
                    std::sync::Arc::new(std::sync::RwLock::new(core_filter)),
                    flags,
                )
            }
            None => FilterType::CoreAllTxs,
        };

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
                    self.process_historical_transactions_from_hash(&hash, count as usize, &filter, tx_hist)
                        .await?;
                }
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(height) => {
                    debug!(height, count, "transactions_with_proofs=historical_from_height_request");
                    self.process_historical_transactions_from_height(
                        height as usize,
                        count as usize,
                        &filter,
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
        let tx_subscription_handle = self
            .subscriber_manager
            .add_subscription(filter.clone())
            .await;
        let subscriber_id = tx_subscription_handle.id().to_string();
        debug!(
            subscriber_id,
            "transactions_with_proofs=subscription_created"
        );
        debug!(
            "Started transaction subscription: {}",
            tx_subscription_handle.id()
        );

        let merkle_block_subscription_handle = self
            .subscriber_manager
            .add_subscription(FilterType::CoreAllBlocks)
            .await;

        debug!(
            subscriber_id = merkle_block_subscription_handle.id(),
            "transactions_with_proofs=merkle_subscription_created"
        );

        // Spawn task to convert internal messages to gRPC responses
        let live_filter = filter.clone();
        let tx_live = tx.clone();
        tokio::spawn(async move {
            trace!(
                subscriber_id = tx_subscription_handle.id(),
                "transactions_with_proofs=worker_started"
            );
            loop {
                let (received, sub_id) = tokio::select! {
                    biased;
                    msg = tx_subscription_handle.recv() => (msg, tx_subscription_handle.id()),
                    msg  = merkle_block_subscription_handle.recv() => (msg, merkle_block_subscription_handle.id()),
                };

                if let Some(message) = received {
                    let response = match message {
                        StreamingEvent::CoreRawTransaction { data: tx_data } => {
                            trace!(
                                subscriber_id = sub_id,
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
                                subscriber_id = sub_id,
                                payload_size = data.len(),
                                "transactions_with_proofs=forward_merkle_block"
                            );
                            // Build merkle block using subscriber's filter
                            let resp = match &live_filter {
                            FilterType::CoreAllTxs => {
                                // All transactions match: construct match flags accordingly
                                if let Ok(block) = dashcore_rpc::dashcore::consensus::encode::deserialize::<dashcore_rpc::dashcore::Block>(&data) {
                                    let match_flags = vec![true; block.txdata.len()];
                                    let mb = build_merkle_block_bytes(&block, &match_flags)
                                        .unwrap_or_else(|e| {
                                            warn!(subscriber_id = sub_id, error = %e, "live_merkle_build_failed_fallback_raw_block");
                                            dashcore_rpc::dashcore::consensus::encode::serialize(&block)
                                        });
                                    TransactionsWithProofsResponse {
                                        responses: Some(
                                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(mb),
                                        ),
                                    }
                                } else {
                                    TransactionsWithProofsResponse {
                                        responses: Some(
                                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data),
                                        ),
                                    }
                                }
                            }
                            FilterType::CoreBloomFilter(bloom, flags) => {
                                if let Ok(block) = dashcore_rpc::dashcore::consensus::encode::deserialize::<dashcore_rpc::dashcore::Block>(&data) {
                                    let mut match_flags = Vec::with_capacity(block.txdata.len());
                                    for tx in block.txdata.iter() {
                                        let mut guard = bloom.write().unwrap();
                                        let m = super::bloom::matches_transaction(&mut guard, tx, *flags);
                                        match_flags.push(m);
                                    }
                                    let mb = build_merkle_block_bytes(&block, &match_flags)
                                        .unwrap_or_else(|e| {
                                            warn!(subscriber_id = sub_id, error = %e, "live_merkle_build_failed_fallback_raw_block");
                                            dashcore_rpc::dashcore::consensus::encode::serialize(&block)
                                        });
                                    TransactionsWithProofsResponse {
                                        responses: Some(
                                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(mb),
                                        ),
                                    }
                                } else {
                                    TransactionsWithProofsResponse {
                                        responses: Some(
                                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data),
                                        ),
                                    }
                                }
                            }
                            _ => TransactionsWithProofsResponse {
                                responses: Some(
                                    dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data),
                                ),
                            },
                        };

                            Ok(resp)
                        }
                        StreamingEvent::CoreInstantLock { data } => {
                            trace!(
                                subscriber_id = sub_id,
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
                                subscriber_id = sub_id,
                                event = ?message,
                                "transactions_with_proofs=ignore_event"
                            );
                            // Ignore other message types for this subscription
                            continue;
                        }
                    };

                    if tx_live.send(response).is_err() {
                        debug!(
                            subscriber_id = sub_id,
                            "transactions_with_proofs=client_disconnected"
                        );
                        break;
                    }
                }
            }
            // Drop of the handle will remove the subscription automatically
            debug!("transactions_with_proofs=worker_finished");
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
                            &filter,
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
                            &filter,
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
        filter: &FilterType,
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
        self.process_historical_transactions_from_height(start_height, count, filter, tx)
            .await
    }

    /// Process historical transactions from a specific block height
    async fn process_historical_transactions_from_height(
        &self,
        from_height: usize,
        count: usize,
        filter: &FilterType,
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
            let block = match self.core_client.get_block_by_hash(hash).await {
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

            trace!(
                height,
                n_txs = txs_bytes.len(),
                "transactions_with_proofs=block_fetched"
            );

            // Track matching transactions and positions to build a merkle block
            let mut matching: Vec<Vec<u8>> = Vec::new();
            let mut match_flags: Vec<bool> = Vec::with_capacity(txs_bytes.len());
            for tx_bytes in txs_bytes.iter() {
                // Try to parse each transaction individually; fallback to contains() if parsing fails
                let matches = match &filter {
                    FilterType::CoreAllTxs => true,
                    FilterType::CoreBloomFilter(bloom, flags) => {
                        match deserialize::<CoreTx>(tx_bytes.as_slice()) {
                            Ok(tx) => {
                                trace!(height, txid = %tx.txid(), "transactions_with_proofs=bloom_matched");
                                let mut core_filter = bloom.write().unwrap();
                                super::bloom::matches_transaction(&mut core_filter, &tx, *flags)
                            }
                            Err(e) => {
                                warn!(height, error = %e, "transactions_with_proofs=tx_deserialize_failed, skipping tx");
                                trace!(height, "transactions_with_proofs=bloom_contains");
                                let core_filter = bloom.read().unwrap();
                                core_filter.contains(tx_bytes)
                            }
                        }
                    }
                    _ => false,
                };
                match_flags.push(matches);
                if matches {
                    matching.push(tx_bytes.clone());
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

            // Then, send a proper merkle block for this height (header + partial merkle tree)
            let merkle_block_bytes = build_merkle_block_bytes(&block, &match_flags)
                .unwrap_or_else(|e| {
                    warn!(height, error = %e, "transactions_with_proofs=merkle_build_failed_fallback_raw_block");
                    dashcore_rpc::dashcore::consensus::encode::serialize(&block)
                });

            let response = TransactionsWithProofsResponse {
                responses: Some(
                    dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(merkle_block_bytes),
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

/// Build a serialized MerkleBlock (header + PartialMerkleTree) from full block bytes and
/// a boolean match flag per transaction indicating which txids should be included.
fn build_merkle_block_bytes(block: &Block, match_flags: &[bool]) -> Result<Vec<u8>, String> {
    use core::consensus::encode::serialize;
    use dashcore_rpc::dashcore as core;

    let header = block.header;
    let txids: Vec<core::Txid> = block.txdata.iter().map(|t| t.txid()).collect();
    if txids.len() != match_flags.len() {
        return Err(format!(
            "flags len {} != tx count {}",
            match_flags.len(),
            txids.len()
        ));
    }

    let pmt = core::merkle_tree::PartialMerkleTree::from_txids(&txids, match_flags);
    let mb = core::merkle_tree::MerkleBlock { header, txn: pmt };
    Ok(serialize(&mb))
}
fn parse_bloom_filter(
    bloom_filter: &dapi_grpc::core::v0::BloomFilter,
) -> Result<
    (
        dashcore_rpc::dashcore::bloom::BloomFilter,
        dashcore_rpc::dashcore::bloom::BloomFlags,
    ),
    Status,
> {
    trace!(
        n_hash_funcs = bloom_filter.n_hash_funcs,
        n_tweak = bloom_filter.n_tweak,
        v_data_len = bloom_filter.v_data.len(),
        v_data = hex::encode(&bloom_filter.v_data),
        "transactions_with_proofs=request_bloom_filter_parsed"
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
    let flags = bloom_flags_from_int(bloom_filter_clone.n_flags);
    let core_filter = dashcore_rpc::dashcore::bloom::BloomFilter::from_bytes(
        bloom_filter_clone.v_data.clone(),
        bloom_filter_clone.n_hash_funcs,
        bloom_filter_clone.n_tweak,
        flags,
    )
    .map_err(|e| Status::invalid_argument(format!("invalid bloom filter data: {}", e)))?;

    Ok((core_filter, flags))
}
