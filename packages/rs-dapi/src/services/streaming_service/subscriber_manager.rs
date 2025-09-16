use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, RwLock, mpsc};
use tracing::{debug, trace, warn};

use crate::clients::tenderdash_websocket::{BlockEvent, TransactionEvent};
use dashcore_rpc::dashcore::bloom::{BloomFilter as CoreBloomFilter, BloomFlags};
use dashcore_rpc::dashcore::{Transaction as CoreTx, consensus::encode::deserialize};

/// Unique identifier for a subscription
pub type SubscriptionId = String;

/// Types of filters supported by the streaming service
#[derive(Debug, Clone)]
pub enum FilterType {
    /// Bloom filter for transaction matching with update flags; filter is persisted/mutable
    CoreBloomFilter(Arc<std::sync::RwLock<CoreBloomFilter>>, BloomFlags),
    /// All platform transactions (Tenderdash)
    PlatformAllTxs,
    /// All Tenderdash platform blocks
    PlatformAllBlocks,
    /// Single platform transaction by uppercase hex hash
    PlatformTxId(String),
    /// All blocks filter (no filtering)
    CoreAllBlocks,
    /// All masternodes filter (no filtering)
    CoreAllMasternodes,
    /// Chain lock events only
    CoreChainLocks,
    /// New Core block hash notifications (for cache invalidation)
    CoreNewBlockHash,
}

/// Subscription information for a streaming client
#[derive(Debug)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub filter: FilterType,
    pub sender: mpsc::UnboundedSender<StreamingEvent>,
}

/// RAII handle: dropping the last clone removes the subscription.
#[derive(Clone)]
pub struct SubscriptionHandle<T>(Arc<SubscriptionHandleInner<T>>);

impl<T> SubscriptionHandle<T> {
    pub fn id(&self) -> &str {
        &self.0.id
    }
}

struct SubscriptionHandleInner<T> {
    subs: Weak<RwLock<BTreeMap<SubscriptionId, Subscription>>>,
    id: SubscriptionId,
    rx: Mutex<mpsc::UnboundedReceiver<T>>, // guarded receiver
}

impl<T> Drop for SubscriptionHandleInner<T> {
    fn drop(&mut self) {
        if let Some(subs) = self.subs.upgrade() {
            let id = self.id.clone();
            tokio::spawn(async move {
                let mut map = subs.write().await;
                if map.remove(&id).is_some() {
                    debug!("Removed subscription (Drop): {}", id);
                }
            });
        }
    }
}

/// Incoming events from various sources to dispatch to subscribers
#[derive(Debug, Clone)]
pub enum StreamingEvent {
    /// Core raw transaction bytes
    CoreRawTransaction { data: Vec<u8> },
    /// Core raw block bytes
    CoreRawBlock { data: Vec<u8> },
    /// Core InstantSend lock
    CoreInstantLock { data: Vec<u8> },
    /// Core ChainLock
    CoreChainLock { data: Vec<u8> },
    /// New block hash event (for side-effects like cache invalidation)
    CoreNewBlockHash { hash: Vec<u8> },
    /// Tenderdash platform transaction event
    PlatformTx { event: TransactionEvent },
    /// Tenderdash platform block event
    PlatformBlock { event: BlockEvent },
    /// Masternode list diff bytes
    CoreMasternodeListDiff { data: Vec<u8> },
}

/// Manages all active streaming subscriptions
#[derive(Debug)]
pub struct SubscriberManager {
    subscriptions: Arc<RwLock<BTreeMap<SubscriptionId, Subscription>>>,
    subscription_counter: AtomicU64,
}

impl SubscriberManager {
    pub fn new() -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(BTreeMap::new())),
            subscription_counter: AtomicU64::new(0),
        }
    }

    /// Add a new subscription and return a handle that can receive messages
    pub async fn add_subscription(&self, filter: FilterType) -> SubscriptionHandle<StreamingEvent> {
        let (sender, receiver) = mpsc::unbounded_channel::<StreamingEvent>();
        let id = self.generate_subscription_id();
        let filter_debug = filter.clone();
        let subscription = Subscription {
            id: id.clone(),
            filter,
            sender,
        };

        self.subscriptions
            .write()
            .await
            .insert(id.clone(), subscription);
        debug!("Added subscription: {}", id);
        trace!(subscription_id = %id, filter = ?filter_debug, "subscription_manager=added");

        SubscriptionHandle(Arc::new(SubscriptionHandleInner::<StreamingEvent> {
            subs: Arc::downgrade(&self.subscriptions),
            id,
            rx: Mutex::new(receiver),
        }))
    }

    /// Remove a subscription
    pub async fn remove_subscription(&self, id: &str) {
        let mut guard = self.subscriptions.write().await;
        if guard.remove(id).is_some() {
            debug!("Removed subscription: {}", id);
            trace!(subscription_id = %id, count_left = guard.len(), "subscription_manager=removed");
        }
    }
}

impl<T> SubscriptionHandle<T> {
    /// Receive the next streaming message for this subscription
    pub async fn recv(&self) -> Option<T> {
        let mut rx = self.0.rx.lock().await;
        rx.recv().await
    }

    /// Map this handle into a new handle of another type by applying `f` to each message.
    /// Consumes the original handle.
    pub fn map<U, F>(self, f: F) -> SubscriptionHandle<U>
    where
        T: Send + 'static,
        U: Send + 'static,
        F: Fn(T) -> U + Send + 'static,
    {
        self.filter_map(move |v| Some(f(v)))
    }

    /// Filter-map: only mapped Some values are forwarded to the new handle. Consumes `self`.
    pub fn filter_map<U, F>(self, f: F) -> SubscriptionHandle<U>
    where
        T: Send + 'static,
        U: Send + 'static,
        F: Fn(T) -> Option<U> + Send + 'static,
    {
        let (tx, rx) = mpsc::unbounded_channel::<U>();
        // Keep original handle alive in the background pump task
        tokio::spawn(async move {
            let this = self;

            loop {
                tokio::select! {
                    biased;
                    _ = tx.closed() => {
                        break;
                    }
                    msg_opt = this.recv() => {
                        match msg_opt {
                            Some(msg) => {
                                if let Some(mapped) = f(msg) {
                                    if tx.send(mapped).is_err() {
                                        break;
                                    }
                                }
                            }
                            None => break,
                        }
                    }
                }
            }
            // dropping `this` will remove the subscription
        });

        SubscriptionHandle(Arc::new(SubscriptionHandleInner::<U> {
            subs: Weak::new(), // mapped handle doesn't own subscription removal
            id: String::from("mapped"),
            rx: Mutex::new(rx),
        }))
    }
}

impl SubscriberManager {
    /// Get the number of active subscriptions
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Unified notify entrypoint routing events to subscribers based on the filter
    pub async fn notify(&self, event: StreamingEvent) {
        let subscriptions = self.subscriptions.read().await;

        trace!(
            active_subscriptions = subscriptions.len(),
            event = ?event,
            "subscription_manager=notify_start"
        );

        let mut dead_subs = vec![];
        for (id, subscription) in subscriptions.iter() {
            if Self::event_matches_filter(&subscription.filter, &event) {
                if let Err(e) = subscription.sender.send(event.clone()) {
                    dead_subs.push(id.clone());
                    warn!(
                        "Failed to send event to subscription {}: {}; removing subscription",
                        subscription.id, e
                    );
                }
            } else {
                trace!(subscription_id = %id, "subscription_manager=filter_no_match");
            }
        }
        drop(subscriptions); // release read lock before acquiring write lock

        // Clean up dead subscriptions
        for sub in dead_subs.iter() {
            self.remove_subscription(sub).await;
        }
    }

    /// Generate a unique subscription ID
    fn generate_subscription_id(&self) -> SubscriptionId {
        let counter = self.subscription_counter.fetch_add(1, Ordering::SeqCst);
        format!("sub_{}", counter)
    }

    /// Check if data matches the subscription filter
    fn core_tx_matches_filter(filter: &FilterType, raw_tx: &[u8]) -> bool {
        match filter {
            FilterType::CoreBloomFilter(f_lock, flags) => match deserialize::<CoreTx>(raw_tx) {
                Ok(tx) => match f_lock.write() {
                    Ok(mut guard) => super::bloom::matches_transaction(&mut guard, &tx, *flags),
                    Err(_) => false,
                },
                Err(_) => match f_lock.read() {
                    Ok(guard) => guard.contains(raw_tx),
                    Err(_) => false,
                },
            },
            _ => false,
        }
    }

    fn event_matches_filter(filter: &FilterType, event: &StreamingEvent) -> bool {
        use StreamingEvent::*;
        let matched = match (filter, event) {
            (FilterType::PlatformAllTxs, PlatformTx { .. }) => true,
            (FilterType::PlatformTxId(id), PlatformTx { event }) => &event.hash == id,
            (FilterType::PlatformAllBlocks, PlatformBlock { .. }) => true,
            (FilterType::CoreNewBlockHash, CoreNewBlockHash { .. }) => true,
            (FilterType::CoreAllBlocks, CoreRawBlock { .. }) => true,
            (FilterType::CoreAllBlocks, CoreChainLock { .. }) => true,
            (FilterType::CoreBloomFilter(_, _), CoreRawTransaction { data }) => {
                Self::core_tx_matches_filter(filter, data)
            }
            (FilterType::CoreBloomFilter(_, _), CoreRawBlock { .. }) => true,
            (FilterType::CoreBloomFilter(_, _), CoreInstantLock { .. }) => true,
            (FilterType::CoreAllMasternodes, CoreMasternodeListDiff { .. }) => true,
            (FilterType::CoreChainLocks, CoreChainLock { .. }) => true,
            _ => false,
        };
        trace!(filter = ?filter, event = ?event, matched, "subscription_manager=filter_evaluated");
        matched
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
    use tokio::time::{Duration, timeout};

    #[tokio::test]
    async fn test_subscription_management() {
        let manager = SubscriberManager::new();

        let handle = manager.add_subscription(FilterType::CoreAllBlocks).await;

        assert_eq!(manager.subscription_count().await, 1);

        manager.remove_subscription(handle.id()).await;
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

        let handle = manager.add_subscription(filter).await;

        // Send non-transaction bytes
        let payload = vec![1u8, 2, 3, 4, 5, 6, 7, 8];
        manager
            .notify(StreamingEvent::CoreRawTransaction {
                data: payload.clone(),
            })
            .await;

        // We should receive one transaction message with the same bytes
        let msg = timeout(Duration::from_millis(200), handle.recv())
            .await
            .expect("timed out")
            .expect("channel closed");

        match msg {
            StreamingEvent::CoreRawTransaction { data } => {
                assert_eq!(data, payload);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_bloom_update_persistence_across_messages_fails_currently() {
        // This test describes desired behavior and is expected to FAIL with the current
        // implementation because filter updates are not persisted (filter is cloned per check).
        let manager = SubscriberManager::new();

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

        let handle = manager.add_subscription(filter).await;

        // Notify with TX A (should match by output pushdata)
        let tx_a_bytes = serialize(&tx_a);
        manager
            .notify(StreamingEvent::CoreRawTransaction {
                data: tx_a_bytes.clone(),
            })
            .await;
        let _first = timeout(Duration::from_millis(200), handle.recv())
            .await
            .expect("timed out waiting for first match")
            .expect("channel closed");

        // Notify with TX B: desired behavior is to match due to persisted outpoint update
        let tx_b_bytes = serialize(&tx_b);
        manager
            .notify(StreamingEvent::CoreRawTransaction {
                data: tx_b_bytes.clone(),
            })
            .await;

        // Expect a second message (this will FAIL until persistence is implemented)
        let _second = timeout(Duration::from_millis(400), handle.recv())
            .await
            .expect("timed out waiting for second match (persistence missing?)")
            .expect("channel closed");
    }
}
