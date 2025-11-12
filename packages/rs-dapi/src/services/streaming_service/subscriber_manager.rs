use dpp::dashcore::prelude::DisplayHex;
use hex::encode;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::{debug, trace};

use crate::clients::tenderdash_websocket::{BlockEvent, TransactionEvent};
use dash_event_bus::event_bus::{
    EventBus, Filter as EventBusFilter, SubscriptionHandle as EventBusSubscriptionHandle,
};
use dashcore_rpc::dashcore::bloom::{BloomFilter as CoreBloomFilter, BloomFlags};
use dashcore_rpc::dashcore::{Transaction as CoreTx, consensus::encode::deserialize};

/// Types of filters supported by the streaming service
#[derive(Debug, Clone)]
pub enum FilterType {
    /// Bloom filter for transaction matching with update flags; filter is persisted/mutable
    CoreBloomFilter(Arc<std::sync::RwLock<CoreBloomFilter>>, BloomFlags),
    /// All Core transactions (no filtering)
    CoreAllTxs,
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

impl FilterType {
    fn matches_core_transaction(&self, raw_tx: &[u8]) -> bool {
        match self {
            FilterType::CoreBloomFilter(bloom, flags) => match deserialize::<CoreTx>(raw_tx) {
                Ok(tx) => super::bloom::matches_transaction(Arc::clone(bloom), &tx, *flags),

                Err(e) => {
                    debug!(
                        error = %e,
                        "Failed to deserialize core transaction for bloom filter matching, falling back to contains()"
                    );
                    match bloom.read() {
                        Ok(guard) => guard.contains(raw_tx),
                        Err(_) => {
                            debug!("Failed to acquire read lock for bloom filter");
                            false
                        }
                    }
                }
            },
            _ => false,
        }
    }

    fn matches_event(&self, event: &StreamingEvent) -> bool {
        use StreamingEvent::*;

        let matched = match (self, event) {
            (FilterType::PlatformAllTxs, PlatformTx { .. }) => true,
            (FilterType::PlatformAllTxs, _) => false,
            (FilterType::PlatformTxId(id), PlatformTx { event }) => &event.hash == id,
            (FilterType::PlatformTxId(_), _) => false,
            (FilterType::PlatformAllBlocks, PlatformBlock { .. }) => true,
            (FilterType::PlatformAllBlocks, _) => false,
            (FilterType::CoreNewBlockHash, CoreNewBlockHash { .. }) => true,
            (FilterType::CoreNewBlockHash, _) => false,
            (FilterType::CoreAllBlocks, CoreRawBlock { .. }) => true,
            (FilterType::CoreAllBlocks, _) => false,
            (FilterType::CoreBloomFilter(_, _), CoreRawTransaction { data }) => {
                self.matches_core_transaction(data)
            }
            (FilterType::CoreBloomFilter(_, _), CoreRawBlock { .. }) => true,
            (FilterType::CoreBloomFilter(_, _), CoreInstantLock { tx_bytes, .. }) => tx_bytes
                .as_ref()
                .map(|data| self.matches_core_transaction(data))
                .unwrap_or(true),
            (FilterType::CoreBloomFilter(_, _), CoreChainLock { .. }) => true,
            (FilterType::CoreBloomFilter(_, _), _) => false,
            (FilterType::CoreAllMasternodes, CoreMasternodeListDiff { .. }) => true,
            (FilterType::CoreAllMasternodes, _) => false,
            (FilterType::CoreChainLocks, CoreChainLock { .. }) => true,
            (FilterType::CoreChainLocks, _) => false,
            (FilterType::CoreAllTxs, CoreRawTransaction { .. }) => true,
            (FilterType::CoreAllTxs, CoreInstantLock { .. }) => true,
            (FilterType::CoreAllTxs, CoreChainLock { .. }) => true,
            (FilterType::CoreAllTxs, _) => false,
        };
        let event_summary = super::summarize_streaming_event(event);
        trace!(matched, filter = ?self, event = %event_summary, "subscription_manager=filter_evaluated");
        matched
    }
}

impl EventBusFilter<StreamingEvent> for FilterType {
    fn matches(&self, event: &StreamingEvent) -> bool {
        self.matches_event(event)
    }
}

/// Incoming events from various sources to dispatch to subscribers
#[derive(Clone)]
pub enum StreamingEvent {
    /// Core raw transaction bytes
    CoreRawTransaction { data: Vec<u8> },
    /// Core raw block bytes
    CoreRawBlock { data: Vec<u8> },
    /// Core InstantSend lock (transaction bytes optional, lock bytes mandatory)
    CoreInstantLock {
        tx_bytes: Option<Vec<u8>>,
        lock_bytes: Vec<u8>,
    },
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

impl Debug for StreamingEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamingEvent::CoreRawTransaction { data } => {
                write!(
                    f,
                    "CoreRawTransaction {{ data: [{}] }}",
                    data.to_lower_hex_string()
                )
            }
            StreamingEvent::CoreRawBlock { data } => {
                write!(
                    f,
                    "CoreRawBlock {{ data: [{}] }}",
                    data.to_lower_hex_string()
                )
            }
            StreamingEvent::CoreInstantLock {
                tx_bytes,
                lock_bytes,
            } => match tx_bytes {
                Some(tx) => write!(
                    f,
                    "CoreInstantLock {{ tx_bytes: [{}], lock_bytes: [{}] }}",
                    encode(tx),
                    encode(lock_bytes)
                ),
                None => write!(
                    f,
                    "CoreInstantLock {{ tx_bytes: none, lock_bytes: [{}] }}",
                    encode(lock_bytes)
                ),
            },
            StreamingEvent::CoreChainLock { data } => {
                write!(
                    f,
                    "CoreChainLock {{ data: [{}] }}",
                    data.to_lower_hex_string()
                )
            }
            StreamingEvent::CoreNewBlockHash { hash } => {
                write!(
                    f,
                    "CoreNewBlockHash {{ hash: [{}] }}",
                    hash.to_lower_hex_string()
                )
            }
            StreamingEvent::PlatformTx { event } => {
                write!(f, "PlatformTx {{ hash: {} }}", event.hash)
            }
            StreamingEvent::PlatformBlock { .. } => {
                write!(f, "PlatformBlock {{ }}")
            }
            StreamingEvent::CoreMasternodeListDiff { data } => {
                write!(
                    f,
                    "CoreMasternodeListDiff {{ data: [{}] }}",
                    data.to_lower_hex_string()
                )
            }
        }
    }
}

/// Manages all active streaming subscriptions
pub type SubscriberManager = EventBus<StreamingEvent, FilterType>;

pub type SubscriptionHandle = EventBusSubscriptionHandle<StreamingEvent, FilterType>;

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
    async fn test_bloom_update_persistence_across_messages() {
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
