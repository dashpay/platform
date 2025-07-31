use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, trace, warn};

/// Unique identifier for a subscription
pub type SubscriptionId = String;

/// Types of filters supported by the streaming service
#[derive(Debug, Clone)]
pub enum FilterType {
    /// Bloom filter for transaction matching
    BloomFilter {
        data: Vec<u8>,
        hash_funcs: u32,
        tweak: u32,
        flags: u32,
    },
    /// All blocks filter (no filtering)
    AllBlocks,
    /// All masternodes filter (no filtering)
    AllMasternodes,
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
        if let Some(_) = self.subscriptions.write().await.remove(id) {
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
            FilterType::BloomFilter {
                data: filter_data,
                hash_funcs,
                tweak,
                flags,
            } => {
                // TODO: Implement proper bloom filter matching
                // For now, always match to test the pipeline
                true
            }
            FilterType::AllBlocks => true,
            FilterType::AllMasternodes => true,
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

    #[tokio::test]
    async fn test_subscription_management() {
        let manager = SubscriberManager::new();
        let (sender, _receiver) = mpsc::unbounded_channel();

        let id = manager
            .add_subscription(
                FilterType::AllBlocks,
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
}
