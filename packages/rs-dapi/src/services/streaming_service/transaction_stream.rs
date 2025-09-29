use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use dapi_grpc::core::v0::transactions_with_proofs_response::Responses;
use dapi_grpc::core::v0::{
    InstantSendLockMessages, RawTransactions, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use dashcore_rpc::dashcore::Block;
use dashcore_rpc::dashcore::hashes::Hash;
use futures::TryFutureExt;
use tokio::sync::{Mutex as AsyncMutex, mpsc, watch};
use tokio::task::JoinSet;
use tokio::time::timeout;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug, trace, warn};

use crate::DapiError;
use crate::clients::{CoreClient, core_client};
use crate::services::streaming_service::{
    FilterType, StreamingEvent, StreamingServiceImpl, SubscriptionHandle,
    bloom::bloom_flags_from_int,
};

const TRANSACTION_STREAM_BUFFER: usize = 512;
/// Maximum duration to keep the delivery gate closed while replaying historical data.
const GATE_MAX_TIMEOUT: Duration = Duration::from_secs(180);

type TxResponseResult = Result<TransactionsWithProofsResponse, Status>;
type TxResponseSender = mpsc::Sender<TxResponseResult>;
type TxResponseStream = ReceiverStream<TxResponseResult>;
type TxResponse = Response<TxResponseStream>;
type DeliveredTxSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveredBlockSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type DeliveredInstantLockSet = Arc<AsyncMutex<HashSet<Vec<u8>>>>;
type GateSender = watch::Sender<bool>;
type GateReceiver = watch::Receiver<bool>;

#[derive(Clone)]
struct TransactionStreamState {
    delivered_txs: DeliveredTxSet,
    delivered_blocks: DeliveredBlockSet,
    delivered_instant_locks: DeliveredInstantLockSet,
    gate_sender: GateSender,
    gate_receiver: GateReceiver,
}

impl TransactionStreamState {
    fn new() -> Self {
        let (gate_sender, gate_receiver) = watch::channel(false);
        Self {
            delivered_txs: Arc::new(AsyncMutex::new(HashSet::new())),
            delivered_blocks: Arc::new(AsyncMutex::new(HashSet::new())),
            delivered_instant_locks: Arc::new(AsyncMutex::new(HashSet::new())),
            gate_sender,
            gate_receiver,
        }
    }

    fn is_gate_open(&self) -> bool {
        *self.gate_receiver.borrow()
    }

    /// Open the gate to allow live events to be processed.
    ///
    /// Provide TransactionStreamState::gate_sender.
    ///
    /// This is decoupled for easier handling between tasks.
    fn open_gate(sender: &GateSender) {
        let _ = sender.send(true);
    }

    async fn wait_for_gate_open(&self) {
        // when true, the gate is already open
        if self.is_gate_open() {
            return;
        }

        let mut receiver = self.gate_receiver.clone();

        let wait_future = async {
            while !*receiver.borrow() {
                if receiver.changed().await.is_err() {
                    break;
                }
            }
        };

        if let Err(e) = timeout(GATE_MAX_TIMEOUT, wait_future).await {
            warn!(
                timeout = GATE_MAX_TIMEOUT.as_secs(),
                "transactions_with_proofs=gate_open_timeout error: {}, forcibly opening gate", e
            );

            Self::open_gate(&self.gate_sender);
        }
    }

    /// Marks a transaction as delivered. Returns false if it was already delivered.
    async fn mark_transaction_delivered(&self, txid: &[u8]) -> bool {
        let mut guard = self.delivered_txs.lock().await;
        guard.insert(txid.to_vec())
    }

    async fn mark_transactions_delivered<I>(&self, txids: I)
    where
        I: IntoIterator<Item = Vec<u8>>,
    {
        let mut guard = self.delivered_txs.lock().await;
        for txid in txids {
            guard.insert(txid);
        }
    }

    async fn mark_block_delivered(&self, block_hash: &[u8]) -> bool {
        let mut guard = self.delivered_blocks.lock().await;
        guard.insert(block_hash.to_vec())
    }

    async fn mark_instant_lock_delivered(&self, instant_lock: &[u8]) -> bool {
        let mut guard = self.delivered_instant_locks.lock().await;
        guard.insert(instant_lock.to_vec())
    }
}

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

        let (tx, rx) = mpsc::channel(TRANSACTION_STREAM_BUFFER);
        if count > 0 {
            // Historical mode
            self.spawn_fetch_transactions_history(
                Some(from_block),
                Some(count as usize),
                filter,
                None,
                tx,
                None,
            )
            .await?;

            debug!("transactions_with_proofs=historical_stream_ready");
        } else {
            self.handle_transactions_combined_mode(from_block, filter, tx)
                .await?;
        }

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn transaction_worker(
        tx_handle: SubscriptionHandle,
        block_handle: SubscriptionHandle,
        tx: TxResponseSender,
        filter: FilterType,
        state: TransactionStreamState,
    ) -> Result<(), DapiError> {
        let subscriber_id = tx_handle.id().to_string();
        let tx_handle_id = tx_handle.id().to_string();
        let block_handle_id = block_handle.id().to_string();

        let mut pending: Vec<(StreamingEvent, String)> = Vec::new();
        // Gate stays closed until historical replay finishes; queue live events until it opens.
        let mut gated = !state.is_gate_open();

        loop {
            tokio::select! {
                _ = state.wait_for_gate_open(), if gated => {
                    gated = !state.is_gate_open();
                    // gated changed from true to false, flush pending events
                    if !gated
                        && !Self::flush_transaction_pending(
                            &filter,
                            &subscriber_id,
                            &tx,
                            &state,
                            &mut pending,
                        ).await {
                            break;
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
                                &state,
                            ).await {
                                tracing::debug!(subscriber_id, block_handle_id, "transactions_with_proofs=forward_block_event_failed");
                                break;
                            }
                        }
                        None => {
                            tracing::debug!(subscriber_id, block_handle_id, "transactions_with_proofs=block_subscription_closed");
                            break
                        },
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
                                &state,
                            ).await {
                                tracing::debug!(subscriber_id, tx_handle_id, "transactions_with_proofs=forward_tx_event_failed");
                                break;
                            }
                        }
                        None => {
                            tracing::debug!(subscriber_id, tx_handle_id, "transactions_with_proofs=tx_subscription_closed");
                            break
                        },
                    }
                }
            }
        }

        debug!(subscriber_id, "transactions_with_proofs=worker_finished");
        Err(DapiError::ConnectionClosed)
    }

    async fn flush_transaction_pending(
        filter: &FilterType,
        subscriber_id: &str,
        tx_sender: &TxResponseSender,
        state: &TransactionStreamState,
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
                state,
            )
            .await
            {
                return false;
            }
        }
        true
    }
    /// Forwards a single transaction-related event to the client if it matches the filter and
    /// has not been previously delivered.
    ///
    /// Returns false if the client has disconnected.
    async fn forward_transaction_event(
        event: StreamingEvent,
        handle_id: &str,
        filter: &FilterType,
        subscriber_id: &str,
        tx_sender: &TxResponseSender,
        state: &TransactionStreamState,
    ) -> bool {
        let maybe_response = match event {
            StreamingEvent::CoreRawTransaction { data } => {
                let Some(txid_bytes) = super::StreamingServiceImpl::txid_bytes_from_bytes(&data)
                else {
                    tracing::debug!("transactions_with_proofs=transaction_no_txid");
                    return true;
                };

                let already_delivered = !state.mark_transaction_delivered(&txid_bytes).await;
                if already_delivered {
                    trace!(
                        subscriber_id,
                        handle_id,
                        txid = %hex::encode(txid_bytes),
                        "transactions_with_proofs=skip_duplicate_transaction"
                    );
                    return true;
                };

                trace!(
                    subscriber_id,
                    handle_id,
                    txid = hex::encode(&txid_bytes),
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

                if block_hash != "n/a"
                    && let Ok(hash_bytes) = hex::decode(&block_hash)
                    && !state.mark_block_delivered(&hash_bytes).await
                {
                    trace!(
                        subscriber_id,
                        handle_id,
                        block_hash = %block_hash,
                        "transactions_with_proofs=skip_duplicate_merkle_block"
                    );
                    return true;
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
                if !state.mark_instant_lock_delivered(&data).await {
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
                    if tx_sender.send(Ok(resp.clone())).await.is_err() {
                        debug!(
                            subscriber_id,
                            "transactions_with_proofs=client_disconnected"
                        );
                        return false;
                    } else {
                        trace!(
                            event = ?resp,
                            subscriber_id,
                            handle_id,
                            "transactions_with_proofs=forward_transaction_event_success"
                        );
                    }
                }
                Err(status) => {
                    let _ = tx_sender.send(Err(status.clone())).await;
                    debug!(
                        subscriber_id,
                        error = %status,
                        "transactions_with_proofs=send_error_to_client"
                    );
                    return false;
                }
            }
        } else {
            trace!(
                subscriber_id,
                handle_id, "transactions_with_proofs=no_response_event"
            );
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
                        match_flags.push(super::bloom::matches_transaction(
                            Arc::clone(bloom),
                            tx,
                            *flags,
                        ));
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

    // Starts a live transaction stream by creating subscriptions for transactions and blocks.
    //
    // Returns the subscriber ID to be used for debugging/logging purposes.
    //
    // Spawns a background task to handle the stream.
    async fn start_live_transaction_stream(
        &self,
        filter: FilterType,
        tx: TxResponseSender,
        state: TransactionStreamState,
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

        self.workers.spawn(async move {
            Self::transaction_worker(
                tx_subscription_handle,
                merkle_block_subscription_handle,
                tx,
                filter,
                state,
            )
            .await
        });

        subscriber_id
    }

    async fn handle_transactions_combined_mode(
        &self,
        from_block: dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock,
        filter: FilterType,
        tx: TxResponseSender,
    ) -> Result<(), Status> {
        let state = TransactionStreamState::new();

        // Will spawn worker thread, gated until historical replay is done
        let subscriber_id = self
            .start_live_transaction_stream(filter.clone(), tx.clone(), state.clone())
            .await;

        // We need our own worker pool so that we can open the gate once historical sync is done
        let mut local_workers = JoinSet::new();

        // Fetch historical transactions in a separate task
        let core_client = self.core_client.clone();

        // this will add new worked to the local_workers pool
        self.spawn_fetch_transactions_history(
            Some(from_block),
            None,
            filter.clone(),
            Some(state.clone()),
            tx.clone(),
            Some(&mut local_workers),
        )
        .await?;

        let gate_sender = state.gate_sender.clone();

        local_workers.spawn(
            Self::fetch_mempool_transactions_worker(filter.clone(), tx.clone(), state, core_client)
                .map_err(DapiError::from),
        );

        // Now, thread that will wait for all local workers  to complete and disable the gate
        let sub_id = subscriber_id.clone();
        self.workers.spawn(async move {
        while let Some(result) = local_workers.join_next().await {
            match result {
                Ok(Ok(())) => { /* task completed successfully */ }
                Ok(Err(e)) => {
                    warn!(error = %e, subscriber_id=&sub_id, "transactions_with_proofs=worker_task_failed");
                    // return error back to caller
                    let status =  e.to_status();
                    let _ = tx.send(Err(status)).await; // ignore returned value
                    return Err(e);
                }
                Err(e) => {
                    warn!(error = %e, subscriber_id=&sub_id, "transactions_with_proofs=worker_task_join_failed");
                    return Err(DapiError::TaskJoin(e));
                }
            }
        }
        TransactionStreamState::open_gate(&gate_sender);
        debug!(subscriber_id=&sub_id, "transactions_with_proofs=historical_sync_completed_gate_opened");

        Ok(())
    });

        debug!(subscriber_id, "transactions_with_proofs=stream_ready");
        Ok(())
    }

    /// Spawns new thread that fetches historical transactions starting from the specified block.
    async fn spawn_fetch_transactions_history(
        &self,
        from_block: Option<dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock>,
        limit: Option<usize>,
        filter: FilterType,
        state: Option<TransactionStreamState>,
        tx: TxResponseSender,
        workers: Option<&mut JoinSet<Result<(), DapiError>>>, // defaults to self.workers if None
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
        let core_client = self.core_client.clone();

        let worker = Self::process_transactions_from_height(
            start_height,
            count_target,
            filter,
            state,
            tx,
            core_client,
        )
        .map_err(DapiError::from);

        if let Some(workers) = workers {
            workers.spawn(worker);
        } else {
            self.workers.spawn(worker);
        }
        Ok(())
    }

    /// Starts fetching mempool transactions that match the filter and sends them to the client.
    ///
    /// Blocking; caller should spawn in a separate task.
    async fn fetch_mempool_transactions_worker(
        filter: FilterType,
        tx: TxResponseSender,
        state: TransactionStreamState,
        core_client: CoreClient,
    ) -> Result<(), Status> {
        use dashcore_rpc::dashcore::consensus::encode::serialize;

        let txids = core_client
            .get_mempool_txids()
            .await
            .map_err(Status::from)?;

        if txids.is_empty() {
            trace!("transactions_with_proofs=mempool_empty");
            return Ok(());
        }

        let mut matching: Vec<Vec<u8>> = Vec::new();

        for txid in txids {
            let tx = match core_client.get_raw_transaction(txid).await {
                Ok(tx) => tx,
                Err(err) => {
                    warn!(error = %err, "transactions_with_proofs=mempool_tx_fetch_failed");
                    continue;
                }
            };

            let matches = match &filter {
                FilterType::CoreAllTxs => true,
                FilterType::CoreBloomFilter(bloom, flags) => {
                    super::bloom::matches_transaction(Arc::clone(bloom), &tx, *flags)
                }
                _ => false,
            };

            if !matches {
                continue;
            }

            let tx_bytes = serialize(&tx);
            let txid_bytes = tx.txid().to_byte_array();

            if !state.mark_transaction_delivered(&txid_bytes).await {
                trace!(
                    txid = %tx.txid(),
                    "transactions_with_proofs=skip_duplicate_mempool_transaction"
                );
                continue;
            }

            matching.push(tx_bytes);
        }

        if matching.is_empty() {
            trace!("transactions_with_proofs=mempool_no_matches");
            return Ok(());
        }

        trace!(
            matches = matching.len(),
            "transactions_with_proofs=forward_mempool_transactions"
        );

        let raw_transactions = RawTransactions {
            transactions: matching,
        };
        if tx
            .send(Ok(TransactionsWithProofsResponse {
                responses: Some(Responses::RawTransactions(raw_transactions)),
            }))
            .await
            .is_err()
        {
            debug!("transactions_with_proofs=mempool_client_disconnected");
        }

        Ok(())
    }

    async fn process_transactions_from_height(
        start_height: usize,
        count: usize,
        filter: FilterType,
        state: Option<TransactionStreamState>,
        tx: TxResponseSender,
        core_client: core_client::CoreClient,
    ) -> Result<(), Status> {
        use dashcore_rpc::dashcore::Transaction as CoreTx;
        use dashcore_rpc::dashcore::consensus::encode::deserialize;

        trace!(
            start_height,
            count, "transactions_with_proofs=historical_begin"
        );

        for i in 0..count {
            let height = (start_height + i) as u32;
            let hash = match core_client.get_block_hash(height).await {
                Ok(h) => h,
                Err(e) => {
                    trace!(height, error = ?e, "transactions_with_proofs=get_block_hash_failed");
                    break;
                }
            };

            let block = match core_client.get_block_by_hash(hash).await {
                Ok(b) => b,
                Err(e) => {
                    trace!(height, error = ?e, "transactions_with_proofs=get_block_raw_with_txs_failed");
                    break;
                }
            };

            let txs_bytes = match core_client.get_block_transactions_bytes_by_hash(hash).await {
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
                                super::bloom::matches_transaction(Arc::clone(bloom), &tx, *flags)
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
                    if let Some(hash_bytes) =
                        super::StreamingServiceImpl::txid_bytes_from_bytes(tx_bytes)
                    {
                        matching_hashes.push(hash_bytes);
                    }
                    matching.push(tx_bytes.clone());
                }
            }

            if !matching.is_empty() {
                if let Some(state) = state.as_ref() {
                    state.mark_transactions_delivered(matching_hashes).await;
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

                if let Some(state) = state.as_ref() {
                    state.mark_block_delivered(&block_hash_bytes).await;
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
            }
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
