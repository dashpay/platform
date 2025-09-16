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

        // Add subscription to manager
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
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_transactions)
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
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data)
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
                                dapi_grpc::core::v0::transactions_with_proofs_response::Responses::InstantSendLockMessages(instant_lock_messages)
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

                if tx.send(response).is_err() {
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

        // Handle historical data if requested
        if count > 0 {
            if let Some(from_block) = req.from_block {
                match from_block {
                    dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHash(hash) => {
                        // TODO: Process historical transactions from block hash
                        debug!(subscriber_id, ?hash, "transactions_with_proofs=historical_from_hash_request");
                        self.process_historical_transactions_from_hash(&hash, count as usize, &bloom_filter_clone)
                            .await?;
                    }
                    dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(height) => {
                        // TODO: Process historical transactions from height
                        debug!(subscriber_id, height, "transactions_with_proofs=historical_from_height_request");
                        self.process_historical_transactions_from_height(
                            height as usize,
                            count as usize,
                            &bloom_filter_clone,
                        )
                        .await?;
                    }
                }
            }
        }

        // Process mempool transactions if count is 0 (streaming mode)
        if req.count == 0 {
            // TODO: Get and filter mempool transactions
            debug!(
                subscriber_id,
                "transactions_with_proofs=streaming_mempool_mode"
            );
        }

        let stream = UnboundedReceiverStream::new(rx);
        debug!(subscriber_id, "transactions_with_proofs=stream_ready");
        Ok(Response::new(stream))
    }

    /// Process historical transactions from a specific block hash
    async fn process_historical_transactions_from_hash(
        &self,
        _from_hash: &[u8],
        _count: usize,
        _bloom_filter: &dapi_grpc::core::v0::BloomFilter,
    ) -> Result<(), Status> {
        // TODO: Implement historical transaction processing from hash
        // This should:
        // 1. Look up the block height for the given hash
        // 2. Fetch the requested number of blocks starting from that height
        // 3. Filter transactions using the bloom filter
        // 4. Send matching transactions to the subscriber
        trace!("transactions_with_proofs=historical_from_hash_unimplemented");
        Ok(())
    }

    /// Process historical transactions from a specific block height
    async fn process_historical_transactions_from_height(
        &self,
        _from_height: usize,
        _count: usize,
        _bloom_filter: &dapi_grpc::core::v0::BloomFilter,
    ) -> Result<(), Status> {
        // TODO: Implement historical transaction processing from height
        // This should:
        // 1. Fetch blocks starting from the given height
        // 2. Extract transactions from each block
        // 3. Filter transactions using the bloom filter
        // 4. Send matching transactions to the subscriber
        trace!("transactions_with_proofs=historical_from_height_unimplemented");
        Ok(())
    }
}
