use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use dapi_grpc::core::v0::transactions_with_proofs_response::Responses;
use dapi_grpc::core::v0::{
    InstantSendLockMessages, RawTransactions, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::Block;
use tokio::sync::{Mutex as AsyncMutex, Notify, mpsc};
use tokio::time::sleep;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, trace, warn};

use crate::services::streaming_service::{
    FilterType, StreamingEvent, StreamingServiceImpl, SubscriptionHandle,
    bloom::bloom_flags_from_int,
};

const TRANSACTION_STREAM_BUFFER: usize = 512;
const HISTORICAL_CORE_QUERY_DELAY: Duration = Duration::from_millis(50);

type TxResponseResult = Result<TransactionsWithProofsResponse, Status>;
type TxResponseSender = mpsc::Sender<TxResponseResult>;
type TxResponseStream = ReceiverStream<TxResponseResult>;
type TxResponse = Response<TxResponseStream>;
type DeliveredTxSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveredBlockSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveredInstantLockSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveryGate = Arc<AtomicBool>;
type DeliveryNotify = Arc<Notify>;

impl StreamingServiceImpl {
    pub async fn subscribe_to_transactions_with_proofs_impl(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<TxResponse, Status> {
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

        let from_block = req
            .from_block
            .ok_or_else(|| Status::invalid_argument("Must specify from_block"))?;

        if count > 0 {
            return self
                .handle_transactions_historical_mode(from_block, count, filter)
                .await;
        }

        self.handle_transactions_combined_mode(from_block, filter)
            .await
    }

    async fn transaction_worker(
        tx_handle: SubscriptionHandle,
        block_handle: SubscriptionHandle,
        tx: TxResponseSender,
        filter: FilterType,
        delivered_txs: DeliveredTxSet,
        delivered_blocks: DeliveredBlockSet,
        delivered_instant_locks: DeliveredInstantLockSet,
        delivery_gate: DeliveryGate,
        delivery_notify: DeliveryNotify,
    ) {
        let subscriber_id = tx_handle.id().to_string();
        let tx_handle_id = tx_handle.id().to_string();
        let block_handle_id = block_handle.id().to_string();

        let mut pending: Vec<(StreamingEvent, String)> = Vec::new();
        let mut gated = !delivery_gate.load(Ordering::Acquire);

        loop {
            tokio::select! {
                _ = delivery_notify.notified(), if gated => {
                    gated = !delivery_gate.load(Ordering::Acquire);
                    if !gated {
                        if !Self::flush_transaction_pending(
                            &filter,
                            &subscriber_id,
                            &tx,
                            &delivered_txs,
                            &delivered_blocks,
                            &delivered_instant_locks,
                            &mut pending,
                        ).await {
                            break;
                        }
                    }
                }
                message = block_handle.recv() => {
                    match message {
                        Some(event) => {
                            if gated {
                                pending.push((event, block_handle_id.clone()));
                                continue;
                            }
                            if !Self::forward_transaction_event(
                                event,
                                &block_handle_id,
                                &filter,
                                &subscriber_id,
                                &tx,
                                &delivered_txs,
                                &delivered_blocks,
                                &delivered_instant_locks,
                            ).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                message = tx_handle.recv() => {
                    match message {
                        Some(event) => {
                            if gated {
                                pending.push((event, tx_handle_id.clone()));
                                continue;
                            }
                            if !Self::forward_transaction_event(
                                event,
                                &tx_handle_id,
                                &filter,
                                &subscriber_id,
                                &tx,
                                &delivered_txs,
                                &delivered_blocks,
                                &delivered_instant_locks,
                            ).await {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        debug!(subscriber_id, "transactions_with_proofs=worker_finished");
    }

    async fn flush_transaction_pending(
        filter: &FilterType,
        subscriber_id: &str,
        tx_sender: &TxResponseSender,
        delivered_txs: &DeliveredTxSet,
        delivered_blocks: &DeliveredBlockSet,
        delivered_instant_locks: &DeliveredInstantLockSet,
        pending: &mut Vec<(StreamingEvent, String)>,
    ) -> bool {
        if pending.is_empty() {
            return true;
        }

        let queued: Vec<(StreamingEvent, String)> = pending.drain(..).collect();
        for (event, handle_id) in queued {
            if !Self::forward_transaction_event(
                event,
                &handle_id,
                filter,
                subscriber_id,
                tx_sender,
                delivered_txs,
                delivered_blocks,
                delivered_instant_locks,
            )
            .await
            {
                return false;
            }
        }
        true
    }

    async fn forward_transaction_event(
        event: StreamingEvent,
        handle_id: &str,
        filter: &FilterType,
        subscriber_id: &str,
        tx_sender: &TxResponseSender,
        delivered_txs: &DeliveredTxSet,
        delivered_blocks: &DeliveredBlockSet,
        delivered_instant_locks: &DeliveredInstantLockSet,
    ) -> bool {
        let maybe_response = match event {
            StreamingEvent::CoreRawTransaction { data } => {
                let txid_hex = super::StreamingServiceImpl::txid_hex_from_bytes(&data);
                if let Some(ref hex_str) = txid_hex {
                    if let Ok(hash_bytes) = hex::decode(hex_str) {
                        let mut guard = delivered_txs.lock().await;
                        if !guard.insert(hash_bytes) {
                            trace!(
                                subscriber_id,
                                handle_id,
                                txid = %hex_str,
                                "transactions_with_proofs=skip_duplicate_transaction"
                            );
                            return true;
                        }
                    }
                }

                let txid_display = txid_hex.unwrap_or_else(|| "n/a".to_string());
                trace!(
                    subscriber_id,
                    handle_id,
                    txid = %txid_display,
                    payload_size = data.len(),
                    "transactions_with_proofs=forward_raw_transaction"
                );
                let raw_transactions = RawTransactions {
                    transactions: vec![data],
                };
                Some(Ok(TransactionsWithProofsResponse {
                    responses: Some(Responses::RawTransactions(raw_transactions)),
                }))
            }
            StreamingEvent::CoreRawBlock { data } => {
                let block_hash =
                    super::StreamingServiceImpl::block_hash_hex_from_block_bytes(&data)
                        .unwrap_or_else(|| "n/a".to_string());

                if block_hash != "n/a" {
                    if let Ok(hash_bytes) = hex::decode(&block_hash) {
                        let mut guard = delivered_blocks.lock().await;
                        if !guard.insert(hash_bytes) {
                            trace!(
                                subscriber_id,
                                handle_id,
                                block_hash = %block_hash,
                                "transactions_with_proofs=skip_duplicate_merkle_block"
                            );
                            return true;
                        }
                    }
                }

                trace!(
                    subscriber_id,
                    handle_id,
                    block_hash = %block_hash,
                    payload_size = data.len(),
                    "transactions_with_proofs=forward_merkle_block"
                );

                match Self::build_transaction_merkle_response(filter, &data, handle_id) {
                    Ok(resp) => Some(Ok(resp)),
                    Err(e) => Some(Err(e)),
                }
            }
            StreamingEvent::CoreInstantLock { data } => {
                let mut guard = delivered_instant_locks.lock().await;
                if !guard.insert(data.clone()) {
                    trace!(
                        subscriber_id,
                        handle_id, "transactions_with_proofs=skip_duplicate_instant_lock"
                    );
                    return true;
                }

                trace!(
                    subscriber_id,
                    handle_id,
                    payload_size = data.len(),
                    "transactions_with_proofs=forward_instant_lock"
                );
                let instant_lock_messages = InstantSendLockMessages {
                    messages: vec![data],
                };
                Some(Ok(TransactionsWithProofsResponse {
                    responses: Some(Responses::InstantSendLockMessages(instant_lock_messages)),
                }))
            }
            other => {
                let summary = super::StreamingServiceImpl::summarize_streaming_event(&other);
                trace!(subscriber_id, handle_id, event = %summary, "transactions_with_proofs=ignore_event");
                None
            }
        };

        if let Some(response) = maybe_response {
            match response {
                Ok(resp) => {
                    if tx_sender.send(Ok(resp)).await.is_err() {
                        debug!(
                            subscriber_id,
                            "transactions_with_proofs=client_disconnected"
                        );
                        return false;
                    }
                }
                Err(status) => {
                    let _ = tx_sender.send(Err(status.clone())).await;
                    return false;
                }
            }
        }

        true
    }

    fn build_transaction_merkle_response(
        filter: &FilterType,
        raw_block: &[u8],
        handle_id: &str,
    ) -> Result<TransactionsWithProofsResponse, Status> {
        use dashcore_rpc::dashcore::consensus::encode::{deserialize, serialize};

        let response = match filter {
            FilterType::CoreAllTxs => {
                if let Ok(block) = deserialize::<Block>(raw_block) {
                    let match_flags = vec![true; block.txdata.len()];
                    let bytes = build_merkle_block_bytes(&block, &match_flags).unwrap_or_else(|e| {
                        warn!(handle_id, error = %e, "transactions_with_proofs=live_merkle_build_failed_fallback_raw_block");
                        serialize(&block)
                    });
                    TransactionsWithProofsResponse {
                        responses: Some(Responses::RawMerkleBlock(bytes)),
                    }
                } else {
                    TransactionsWithProofsResponse {
                        responses: Some(Responses::RawMerkleBlock(raw_block.to_vec())),
                    }
                }
            }
            FilterType::CoreBloomFilter(bloom, flags) => {
                if let Ok(block) = deserialize::<Block>(raw_block) {
                    let mut match_flags = Vec::with_capacity(block.txdata.len());
                    for tx in block.txdata.iter() {
                        let mut guard = bloom.write().unwrap();
                        match_flags.push(super::bloom::matches_transaction(&mut guard, tx, *flags));
                    }
                    let bytes = build_merkle_block_bytes(&block, &match_flags).unwrap_or_else(|e| {
                        warn!(handle_id, error = %e, "transactions_with_proofs=live_merkle_build_failed_fallback_raw_block");
                        serialize(&block)
                    });
                    TransactionsWithProofsResponse {
                        responses: Some(Responses::RawMerkleBlock(bytes)),
                    }
                } else {
                    TransactionsWithProofsResponse {
                        responses: Some(Responses::RawMerkleBlock(raw_block.to_vec())),
                    }
                }
            }
            _ => TransactionsWithProofsResponse {
                responses: Some(Responses::RawMerkleBlock(raw_block.to_vec())),
            },
        };

        Ok(response)
    }

    async fn start_live_transaction_stream(
        &self,
        filter: FilterType,
        tx: TxResponseSender,
        delivered_txs: DeliveredTxSet,
        delivered_blocks: DeliveredBlockSet,
        delivered_instant_locks: DeliveredInstantLockSet,
        delivery_gate: DeliveryGate,
        delivery_notify: DeliveryNotify,
    ) -> String {
        let tx_subscription_handle = self
            .subscriber_manager
            .add_subscription(filter.clone())
            .await;
        let subscriber_id = tx_subscription_handle.id().to_string();
        debug!(
            subscriber_id,
            "transactions_with_proofs=subscription_created"
        );

        let merkle_block_subscription_handle = self
            .subscriber_manager
            .add_subscription(FilterType::CoreAllBlocks)
            .await;

        debug!(
            subscriber_id = merkle_block_subscription_handle.id(),
            "transactions_with_proofs=merkle_subscription_created"
        );

        let workers = self.workers.clone();
        workers.spawn(async move {
            Self::transaction_worker(
                tx_subscription_handle,
                merkle_block_subscription_handle,
                tx,
                filter,
                delivered_txs,
                delivered_blocks,
                delivered_instant_locks,
                delivery_gate,
                delivery_notify,
            )
            .await;
            Ok::<(), ()>(())
        });

        subscriber_id
    }

    async fn handle_transactions_historical_mode(
        &self,
        from_block: dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock,
        count: u32,
        filter: FilterType,
    ) -> Result<TxResponse, Status> {
        let (tx, rx) = mpsc::channel(TRANSACTION_STREAM_BUFFER);
        self.fetch_transactions_history(
            Some(from_block),
            Some(count as usize),
            filter,
            None,
            None,
            None,
            tx.clone(),
        )
        .await?;

        debug!("transactions_with_proofs=historical_stream_ready");
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn handle_transactions_combined_mode(
        &self,
        from_block: dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock,
        filter: FilterType,
    ) -> Result<TxResponse, Status> {
        let (tx, rx) = mpsc::channel(TRANSACTION_STREAM_BUFFER);
        let delivered_txs: DeliveredTxSet = Arc::new(AsyncMutex::new(HashSet::new()));
        let delivered_blocks: DeliveredBlockSet = Arc::new(AsyncMutex::new(HashSet::new()));
        let delivered_instant_locks: DeliveredInstantLockSet =
            Arc::new(AsyncMutex::new(HashSet::new()));
        let delivery_gate: DeliveryGate = Arc::new(AtomicBool::new(false));
        let delivery_notify = Arc::new(Notify::new());

        let subscriber_id = self
            .start_live_transaction_stream(
                filter.clone(),
                tx.clone(),
                delivered_txs.clone(),
                delivered_blocks.clone(),
                delivered_instant_locks.clone(),
                delivery_gate.clone(),
                delivery_notify.clone(),
            )
            .await;

        self.fetch_transactions_history(
            Some(from_block),
            None,
            filter.clone(),
            Some(delivered_txs.clone()),
            Some(delivered_blocks.clone()),
            Some(delivered_instant_locks.clone()),
            tx.clone(),
        )
        .await?;

        delivery_gate.store(true, Ordering::Release);
        delivery_notify.notify_waiters();

        debug!(subscriber_id, "transactions_with_proofs=stream_ready");
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn fetch_transactions_history(
        &self,
        from_block: Option<dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock>,
        limit: Option<usize>,
        filter: FilterType,
        delivered_txs: Option<DeliveredTxSet>,
        delivered_blocks: Option<DeliveredBlockSet>,
        delivered_instant_locks: Option<DeliveredInstantLockSet>,
        tx: TxResponseSender,
    ) -> Result<(), Status> {
        use std::str::FromStr;

        let from_block = match from_block {
            Some(block) => block,
            None => return Ok(()),
        };

        let best_height = self
            .core_client
            .get_block_count()
            .await
            .map_err(Status::from)? as usize;

        let (start_height, count_target) = match from_block {
            dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHash(
                hash,
            ) => {
                let hash_hex = hex::encode(&hash);
                let block_hash = dashcore_rpc::dashcore::BlockHash::from_str(&hash_hex)
                    .map_err(|e| Status::invalid_argument(format!("Invalid block hash: {}", e)))?;
                let header = self
                    .core_client
                    .get_block_header_info(&block_hash)
                    .await
                    .map_err(Status::from)?;
                let start = header.height as usize;
                let available = best_height.saturating_sub(start).saturating_add(1);
                let desired = limit.map_or(available, |limit| limit.min(available));
                debug!(
                    start,
                    desired, "transactions_with_proofs=historical_from_hash_request"
                );
                (start, desired)
            }
            dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(
                height,
            ) => {
                let start = height as usize;
                if start == 0 {
                    return Err(Status::invalid_argument(
                        "Minimum value for `fromBlockHeight` is 1",
                    ));
                }
                if start > best_height.saturating_add(1) {
                    return Err(Status::not_found(format!("Block {} not found", start)));
                }
                let available = best_height.saturating_sub(start).saturating_add(1);
                let desired = limit.map_or(available, |limit| limit.min(available));
                debug!(
                    start,
                    desired, "transactions_with_proofs=historical_from_height_request"
                );
                (start, desired)
            }
        };

        if count_target == 0 {
            return Ok(());
        }

        self.process_transactions_from_height(
            start_height,
            count_target,
            filter,
            delivered_txs,
            delivered_blocks,
            delivered_instant_locks,
            tx,
        )
        .await
    }

    async fn process_transactions_from_height(
        &self,
        start_height: usize,
        count: usize,
        filter: FilterType,
        delivered_txs: Option<DeliveredTxSet>,
        delivered_blocks: Option<DeliveredBlockSet>,
        delivered_instant_locks: Option<DeliveredInstantLockSet>,
        tx: TxResponseSender,
    ) -> Result<(), Status> {
        use dashcore_rpc::dashcore::Transaction as CoreTx;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;

        trace!(
            start_height,
            count, "transactions_with_proofs=historical_begin"
        );

        let _ = delivered_instant_locks;

        for i in 0..count {
            let height = (start_height + i) as u32;
            let hash = match self.core_client.get_block_hash(height).await {
                Ok(h) => h,
                Err(e) => {
                    trace!(height, error = ?e, "transactions_with_proofs=get_block_hash_failed");
                    break;
                }
            };

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

            let block_hash_bytes =
                <dashcore_rpc::dashcore::BlockHash as AsRef<[u8]>>::as_ref(&hash).to_vec();

            let mut matching: Vec<Vec<u8>> = Vec::new();
            let mut matching_hashes: Vec<Vec<u8>> = Vec::new();
            let mut match_flags: Vec<bool> = Vec::with_capacity(txs_bytes.len());

            for tx_bytes in txs_bytes.iter() {
                let matches = match &filter {
                    FilterType::CoreAllTxs => true,
                    FilterType::CoreBloomFilter(bloom, flags) => {
                        match deserialize::<CoreTx>(tx_bytes.as_slice()) {
                            Ok(tx) => {
                                trace!(height, txid = %tx.txid(), "transactions_with_proofs=bloom_matched");
                                let mut guard = bloom.write().unwrap();
                                super::bloom::matches_transaction(&mut guard, &tx, *flags)
                            }
                            Err(e) => {
                                warn!(height, error = %e, "transactions_with_proofs=tx_deserialize_failed, skipping tx");
                                let guard = bloom.read().unwrap();
                                guard.contains(tx_bytes)
                            }
                        }
                    }
                    _ => false,
                };
                match_flags.push(matches);
                if matches {
                    if let Some(txid_hex) =
                        super::StreamingServiceImpl::txid_hex_from_bytes(tx_bytes)
                    {
                        if let Ok(bytes) = hex::decode(txid_hex) {
                            matching_hashes.push(bytes);
                        }
                    }
                    matching.push(tx_bytes.clone());
                }
            }

            if !matching.is_empty() {
                if let Some(shared) = delivered_txs.as_ref() {
                    let mut guard = shared.lock().await;
                    for hash_bytes in matching_hashes.iter() {
                        guard.insert(hash_bytes.clone());
                    }
                }

                let raw_transactions = RawTransactions {
                    transactions: matching,
                };
                let response = TransactionsWithProofsResponse {
                    responses: Some(Responses::RawTransactions(raw_transactions)),
                };
                if tx.send(Ok(response)).await.is_err() {
                    debug!("transactions_with_proofs=historical_client_disconnected");
                    return Ok(());
                }
            }

            if let Some(shared) = delivered_blocks.as_ref() {
                let mut guard = shared.lock().await;
                guard.insert(block_hash_bytes.clone());
            }

            let merkle_block_bytes = build_merkle_block_bytes(&block, &match_flags).unwrap_or_else(|e| {
                let bh = block.block_hash();
                warn!(height, block_hash = %bh, error = %e, "transactions_with_proofs=merkle_build_failed_fallback_raw_block");
                dashcore_rpc::dashcore::consensus::encode::serialize(&block)
            });

            let response = TransactionsWithProofsResponse {
                responses: Some(Responses::RawMerkleBlock(merkle_block_bytes)),
            };
            if tx.send(Ok(response)).await.is_err() {
                debug!("transactions_with_proofs=historical_client_disconnected");
                return Ok(());
            }

            sleep(HISTORICAL_CORE_QUERY_DELAY).await;
        }

        trace!(
            start_height,
            count, "transactions_with_proofs=historical_end"
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
        v_data_prefix = %super::StreamingServiceImpl::short_hex(&bloom_filter.v_data, 16),
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
