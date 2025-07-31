use dapi_grpc::core::v0::{
    InstantSendLockMessages, RawTransactions, TransactionsWithProofsRequest,
    TransactionsWithProofsResponse,
};
use dapi_grpc::tonic::{Request, Response, Status};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{debug, info};

use crate::services::streaming_service::subscriber_manager::{
    FilterType, StreamingMessage, SubscriptionType,
};
use crate::services::streaming_service::StreamingServiceImpl;

impl StreamingServiceImpl {
    pub async fn subscribe_to_transactions_with_proofs_impl(
        &self,
        request: Request<TransactionsWithProofsRequest>,
    ) -> Result<
        Response<UnboundedReceiverStream<Result<TransactionsWithProofsResponse, Status>>>,
        Status,
    > {
        let req = request.into_inner();

        // Extract bloom filter parameters
        let bloom_filter = req
            .bloom_filter
            .ok_or_else(|| Status::invalid_argument("bloom_filter is required"))?;

        // Validate bloom filter parameters
        if bloom_filter.v_data.is_empty() {
            return Err(Status::invalid_argument(
                "bloom filter data cannot be empty",
            ));
        }

        if bloom_filter.n_hash_funcs == 0 {
            return Err(Status::invalid_argument(
                "number of hash functions must be greater than 0",
            ));
        }

        // Create filter from bloom filter parameters
        let bloom_filter_clone = bloom_filter.clone();
        let count = req.count;
        let filter = FilterType::BloomFilter {
            data: bloom_filter.v_data,
            hash_funcs: bloom_filter.n_hash_funcs,
            tweak: bloom_filter.n_tweak,
            flags: bloom_filter.n_flags,
        };

        // Create channel for streaming responses
        let (tx, rx) = mpsc::unbounded_channel();

        // Create message channel for internal communication
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<StreamingMessage>();

        // Add subscription to manager
        let subscription_id = self
            .subscriber_manager
            .add_subscription(filter, SubscriptionType::TransactionsWithProofs, message_tx)
            .await;

        info!("Started transaction subscription: {}", subscription_id);

        // Spawn task to convert internal messages to gRPC responses
        let subscriber_manager = self.subscriber_manager.clone();
        let sub_id = subscription_id.clone();
        tokio::spawn(async move {
            while let Some(message) = message_rx.recv().await {
                let response = match message {
                    StreamingMessage::Transaction {
                        tx_data,
                        merkle_proof: _,
                    } => {
                        let mut raw_transactions = RawTransactions::default();
                        raw_transactions.transactions = vec![tx_data];

                        let mut response = TransactionsWithProofsResponse::default();
                        response.responses = Some(
                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawTransactions(raw_transactions)
                        );

                        Ok(response)
                    }
                    StreamingMessage::MerkleBlock { data } => {
                        let mut response = TransactionsWithProofsResponse::default();
                        response.responses = Some(
                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::RawMerkleBlock(data)
                        );

                        Ok(response)
                    }
                    StreamingMessage::InstantLock { data } => {
                        let mut instant_lock_messages = InstantSendLockMessages::default();
                        instant_lock_messages.messages = vec![data];

                        let mut response = TransactionsWithProofsResponse::default();
                        response.responses = Some(
                            dapi_grpc::core::v0::transactions_with_proofs_response::Responses::InstantSendLockMessages(instant_lock_messages)
                        );

                        Ok(response)
                    }
                    _ => {
                        // Ignore other message types for this subscription
                        continue;
                    }
                };

                if let Err(_) = tx.send(response) {
                    debug!(
                        "Client disconnected from transaction subscription: {}",
                        sub_id
                    );
                    break;
                }
            }

            // Clean up subscription when client disconnects
            subscriber_manager.remove_subscription(&sub_id).await;
            info!("Cleaned up transaction subscription: {}", sub_id);
        });

        // Handle historical data if requested
        if count > 0 {
            if let Some(from_block) = req.from_block {
                match from_block {
                    dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHash(hash) => {
                        // TODO: Process historical transactions from block hash
                        debug!(
                            "Historical transaction processing requested from hash: {:?}",
                            hash
                        );
                        self.process_historical_transactions_from_hash(&hash, count as usize, &bloom_filter_clone)
                            .await?;
                    }
                    dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(height) => {
                        // TODO: Process historical transactions from height
                        debug!(
                            "Historical transaction processing requested from height: {}",
                            height
                        );
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
            debug!("Mempool transaction processing requested");
        }

        let stream = UnboundedReceiverStream::new(rx);
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
        debug!("Processing historical transactions from hash not yet implemented");
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
        debug!("Processing historical transactions from height not yet implemented");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clients::mock::{MockDriveClient, MockTenderdashClient};
    use crate::config::Config;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_transaction_subscription_creation() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());

        let service = StreamingServiceImpl::new(drive_client, tenderdash_client, config).unwrap();

        let bloom_filter = dapi_grpc::core::v0::BloomFilter {
            v_data: vec![0xFF, 0x00, 0xFF],
            n_hash_funcs: 3,
            n_tweak: 12345,
            n_flags: 0,
        };

        let request = Request::new(TransactionsWithProofsRequest {
            bloom_filter: Some(bloom_filter),
            from_block: Some(
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(
                    100,
                ),
            ),
            count: 0,
            send_transaction_hashes: false,
        });

        let result = service
            .subscribe_to_transactions_with_proofs_impl(request)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_transaction_subscription_invalid_filter() {
        let config = Arc::new(Config::default());
        let drive_client = Arc::new(MockDriveClient::new());
        let tenderdash_client = Arc::new(MockTenderdashClient::new());

        let service = StreamingServiceImpl::new(drive_client, tenderdash_client, config).unwrap();

        let request = Request::new(TransactionsWithProofsRequest {
            bloom_filter: None, // Missing bloom filter
            from_block: Some(
                dapi_grpc::core::v0::transactions_with_proofs_request::FromBlock::FromBlockHeight(
                    100,
                ),
            ),
            count: 0,
            send_transaction_hashes: false,
        });

        let result = service
            .subscribe_to_transactions_with_proofs_impl(request)
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().code(),
            dapi_grpc::tonic::Code::InvalidArgument
        );
    }
}
