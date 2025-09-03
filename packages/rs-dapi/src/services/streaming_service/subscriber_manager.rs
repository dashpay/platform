use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, trace, warn};

use dashcore_rpc::dashcore::bloom::{BloomFilter as CoreBloomFilter, BloomFlags};
use dashcore_rpc::dashcore::{consensus::encode::deserialize, Transaction as CoreTx};

/// Unique identifier for a subscription
pub type SubscriptionId = String;

/// Types of filters supported by the streaming service
#[derive(Debug, Clone)]
pub enum FilterType {
    /// Bloom filter for transaction matching with update flags; filter is persisted/mutable
    CoreBloomFilter(Arc<std::sync::RwLock<CoreBloomFilter>>, BloomFlags),
    /// All blocks filter (no filtering)
    CoreAllBlocks,
    /// All masternodes filter (no filtering)
    CoreAllMasternodes,
}

/// Subscription information for a streaming client
#[derive(Debug)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub filter: FilterType,
    pub sender: mpsc::UnboundedSender<StreamingMessage>,
    pub subscription_type: SubscriptionType,
}

/// Types of streaming subscriptions
#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionType {
    TransactionsWithProofs,
    BlockHeadersWithChainLocks,
    MasternodeList,
}

/// Messages sent to streaming clients
#[derive(Debug, Clone)]
pub enum StreamingMessage {
    /// Raw transaction data with merkle proof
    Transaction {
        tx_data: Vec<u8>,
        merkle_proof: Option<Vec<u8>>,
    },
    /// Merkle block data
    MerkleBlock { data: Vec<u8> },
    /// InstantSend lock message
    InstantLock { data: Vec<u8> },
    /// Block header data
    BlockHeader { data: Vec<u8> },
    /// Chain lock data
    ChainLock { data: Vec<u8> },
    /// Masternode list diff data
    MasternodeListDiff { data: Vec<u8> },
}

/// Manages all active streaming subscriptions
pub struct SubscriberManager {
    subscriptions: Arc<RwLock<HashMap<SubscriptionId, Subscription>>>,
    subscription_counter: AtomicU64,
}

impl SubscriberManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            subscription_counter: AtomicU64::new(0),
        }
    }

    /// Add a new subscription
    pub async fn add_subscription(
        &self,
        filter: FilterType,
        subscription_type: SubscriptionType,
        sender: mpsc::UnboundedSender<StreamingMessage>,
    ) -> SubscriptionId {
        let id = self.generate_subscription_id();
        let subscription = Subscription {
            id: id.clone(),
            filter,
            sender,
            subscription_type: subscription_type.clone(),
        };

        self.subscriptions
            .write()
            .await
            .insert(id.clone(), subscription);
        debug!("Added subscription: {} of type {:?}", id, subscription_type);

        id
    }

    /// Remove a subscription
    pub async fn remove_subscription(&self, id: &SubscriptionId) {
        if self.subscriptions.write().await.remove(id).is_some() {
            debug!("Removed subscription: {}", id);
        }
    }

    /// Get the number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Notify transaction subscribers with matching filters
    pub async fn notify_transaction_subscribers(&self, tx_data: &[u8]) {
        let subscriptions = self.subscriptions.read().await;
        trace!("Notifying transaction subscribers: {} bytes", tx_data.len());

        for subscription in subscriptions.values() {
            if subscription.subscription_type != SubscriptionType::TransactionsWithProofs {
                continue;
            }

            if self.matches_filter(&subscription.filter, tx_data) {
                let message = StreamingMessage::Transaction {
                    tx_data: tx_data.to_vec(),
                    merkle_proof: None, // TODO: Generate merkle proof
                };

                if let Err(e) = subscription.sender.send(message) {
                    warn!(
                        "Failed to send transaction to subscriber {}: {}",
                        subscription.id, e
                    );
                }
            }
        }
    }

    /// Notify block subscribers
    pub async fn notify_block_subscribers(&self, block_data: &[u8]) {
        let subscriptions = self.subscriptions.read().await;

        for subscription in subscriptions.values() {
            if subscription.subscription_type == SubscriptionType::TransactionsWithProofs {
                // Send merkle block for transaction filtering
                let message = StreamingMessage::MerkleBlock {
                    data: block_data.to_vec(),
                };

                if let Err(e) = subscription.sender.send(message) {
                    warn!(
                        "Failed to send merkle block to subscriber {}: {}",
                        subscription.id, e
                    );
                }
            } else if subscription.subscription_type == SubscriptionType::BlockHeadersWithChainLocks
            {
                // Extract and send block header
                let message = StreamingMessage::BlockHeader {
                    data: self.extract_block_header(block_data),
                };

                if let Err(e) = subscription.sender.send(message) {
                    warn!(
                        "Failed to send block header to subscriber {}: {}",
                        subscription.id, e
                    );
                }
            }
        }
    }

    /// Notify instant lock subscribers
    pub async fn notify_instant_lock_subscribers(&self, lock_data: &[u8]) {
        let subscriptions = self.subscriptions.read().await;

        for subscription in subscriptions.values() {
            if subscription.subscription_type == SubscriptionType::TransactionsWithProofs {
                let message = StreamingMessage::InstantLock {
                    data: lock_data.to_vec(),
                };

                if let Err(e) = subscription.sender.send(message) {
                    warn!(
                        "Failed to send instant lock to subscriber {}: {}",
                        subscription.id, e
                    );
                }
            }
        }
    }

    /// Notify chain lock subscribers
    pub async fn notify_chain_lock_subscribers(&self, lock_data: &[u8]) {
        let subscriptions = self.subscriptions.read().await;

        for subscription in subscriptions.values() {
            if subscription.subscription_type == SubscriptionType::BlockHeadersWithChainLocks {
                let message = StreamingMessage::ChainLock {
                    data: lock_data.to_vec(),
                };

                if let Err(e) = subscription.sender.send(message) {
                    warn!(
                        "Failed to send chain lock to subscriber {}: {}",
                        subscription.id, e
                    );
                }
            }
        }
    }

    /// Notify new block subscribers (hash-based notifications)
    pub async fn notify_new_block_subscribers(&self, _block_hash: &[u8]) {
        // This triggers cache invalidation and other block-related processing
        debug!("New block notification received");
        // TODO: Implement cache invalidation and other block processing
    }

    /// Generate a unique subscription ID
    fn generate_subscription_id(&self) -> SubscriptionId {
        let counter = self.subscription_counter.fetch_add(1, Ordering::SeqCst);
        format!("sub_{}", counter)
    }

    /// Check if data matches the subscription filter
    fn matches_filter(&self, filter: &FilterType, data: &[u8]) -> bool {
        match filter {
            FilterType::CoreBloomFilter(f_lock, flags) => match deserialize::<CoreTx>(data) {
                Ok(tx) => match f_lock.write() {
                    Ok(mut guard) => super::bloom::matches_transaction(&mut guard, &tx, *flags),
                    Err(_) => false,
                },
                Err(_) => match f_lock.read() {
                    Ok(guard) => guard.contains(data),
                    Err(_) => false,
                },
            },
            FilterType::CoreAllBlocks => true,
            FilterType::CoreAllMasternodes => true,
        }
    }

    /// Extract block header from full block data
    fn extract_block_header(&self, block_data: &[u8]) -> Vec<u8> {
        // TODO: Implement proper block header extraction
        // For now, return first 80 bytes (typical block header size)
        if block_data.len() >= 80 {
            block_data[..80].to_vec()
        } else {
            block_data.to_vec()
        }
    }
}

impl Default for SubscriberManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dashcore_rpc::dashcore::bloom::BloomFlags;
    use dashcore_rpc::dashcore::consensus::encode::serialize;
    use dashcore_rpc::dashcore::hashes::Hash;
    use dashcore_rpc::dashcore::{OutPoint, PubkeyHash, ScriptBuf, TxIn, TxOut};
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn test_subscription_management() {
        let manager = SubscriberManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();

        let id = manager
            .add_subscription(
                FilterType::CoreAllBlocks,
                SubscriptionType::BlockHeadersWithChainLocks,
                sender,
            )
            .await;

        assert_eq!(manager.subscription_count().await, 1);

        manager.remove_subscription(&id).await;
        assert_eq!(manager.subscription_count().await, 0);
    }

    #[test]
    fn test_subscription_id_generation() {
        let manager = SubscriberManager::new();
        let id1 = manager.generate_subscription_id();
        let id2 = manager.generate_subscription_id();

        assert_ne!(id1, id2);
        assert!(id1.starts_with("sub_"));
        assert!(id2.starts_with("sub_"));
    }

    #[tokio::test]
    async fn test_non_tx_bytes_fallbacks_to_contains() {
        let manager = SubscriberManager::new();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        // Create a filter with all bits set so contains() returns true for any data
        let filter = FilterType::CoreBloomFilter(
            std::sync::Arc::new(std::sync::RwLock::new(
                dashcore_rpc::dashcore::bloom::BloomFilter::from_bytes(
                    vec![0xFF; 8],
                    5,
                    0,
                    BloomFlags::None,
                )
                .expect("failed to create bloom filter"),
            )),
            BloomFlags::None,
        );

        let _id = manager
            .add_subscription(filter, SubscriptionType::TransactionsWithProofs, sender)
            .await;

        // Send non-transaction bytes
        let payload = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        manager.notify_transaction_subscribers(&payload).await;

        // We should receive one transaction message with the same bytes
        let msg = timeout(Duration::from_millis(200), receiver.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        match msg {
            StreamingMessage::Transaction {
                tx_data,
                merkle_proof: _,
            } => {
                assert_eq!(tx_data, payload);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bloom_update_persistence_across_messages_fails_currently() {
        // This test describes desired behavior and is expected to FAIL with the current
        // implementation because filter updates are not persisted (filter is cloned per check).
        let manager = SubscriberManager::new();
        let (sender, mut receiver) = mpsc::unbounded_channel();

        // Build TX A with a P2PKH output whose hash160 we seed into the filter
        let h160 = PubkeyHash::from_byte_array([0x44; 20]);
        let script_a = ScriptBuf::new_p2pkh(&h160);
        let tx_a = dashcore_rpc::dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![],
            output: vec![TxOut {
                value: 1500,
                script_pubkey: script_a,
            }],
            special_transaction_payload: None,
        };

        // Build TX B spending outpoint (tx_a.txid, vout=0)
        let tx_a_txid = tx_a.txid();
        let tx_b = dashcore_rpc::dashcore::Transaction {
            version: 2,
            lock_time: 0,
            input: vec![TxIn {
                previous_output: OutPoint {
                    txid: tx_a_txid,
                    vout: 0,
                },
                script_sig: ScriptBuf::new(),
                sequence: 0xFFFFFFFF,
                witness: Default::default(),
            }],
            output: vec![],
            special_transaction_payload: None,
        };

        // Subscription with BLOOM_UPDATE_ALL so outpoint should be added after TX A matches
        let mut base_filter = dashcore_rpc::dashcore::bloom::BloomFilter::from_bytes(
            vec![0; 512],
            5,
            12345,
            BloomFlags::All,
        )
        .unwrap();
        base_filter.insert(&h160.to_byte_array());
        let filter = FilterType::CoreBloomFilter(
            std::sync::Arc::new(std::sync::RwLock::new(base_filter)),
            BloomFlags::All,
        );

        let _id = manager
            .add_subscription(filter, SubscriptionType::TransactionsWithProofs, sender)
            .await;

        // Notify with TX A (should match by output pushdata)
        let tx_a_bytes = serialize(&tx_a);
        manager.notify_transaction_subscribers(&tx_a_bytes).await;
        let _first = timeout(Duration::from_millis(200), receiver.recv())
            .await
            .expect("timed out waiting for first match")
            .expect("channel closed");

        // Notify with TX B: desired behavior is to match due to persisted outpoint update
        let tx_b_bytes = serialize(&tx_b);
        manager.notify_transaction_subscribers(&tx_b_bytes).await;

        // Expect a second message (this will FAIL until persistence is implemented)
        let _second = timeout(Duration::from_millis(400), receiver.recv())
            .await
            .expect("timed out waiting for second match (persistence missing?)")
            .expect("channel closed");
    }
}
