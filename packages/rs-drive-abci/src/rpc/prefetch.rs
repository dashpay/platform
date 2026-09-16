//! Speculative fetching of the two Core RPC responses a block needs when the
//! core chain-locked height advances.
//!
//! Replaying mainnet history, roughly every other Platform block advances the
//! core height by one, and each of those blocks blocks on `protx listdiff` and
//! `quorum listextended` in turn — about a millisecond of the seven a block
//! costs. The heights are consecutive, so the answer for the next one can be
//! fetched while the current block is still executing.
//!
//! A guess is only ever made for a chain-locked height. Platform only asks Core
//! about chain-locked blocks, and neither response carries the block hash, so an
//! answer fetched for a block that was later reorged could not be told apart
//! from the final one; a node that prefetched would then apply a different
//! masternode diff than a node that did not. Staying below the chain lock rules
//! that out, and also means a guess cannot fail for want of a block, so there is
//! nothing to back off from. At the tip this costs one `getbestchainlock` per
//! advancing core block; during replay the cached value covers thousands.
//!
//! The speculative call runs on its own connection, so it never sits in front of
//! a real one: `jsonrpc`'s HTTP transport serialises requests behind a single
//! socket mutex.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};

use dpp::dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumListResult, MasternodeListDiff};
use dpp::dashcore_rpc::{Auth, Client, Error as CoreError, RpcApi};

/// The Core calls the prefetcher makes, so the gating logic can be tested
/// without a Core node.
pub(crate) trait PrefetchSource: Send + Sync + 'static {
    fn best_chain_locked_height(&self) -> Result<u32, CoreError>;
    fn quorum_list(&self, height: u32) -> Result<ExtendedQuorumListResult, CoreError>;
    fn protx_diff(&self, base: u32, block: u32) -> Result<MasternodeListDiff, CoreError>;
}

impl PrefetchSource for Client {
    fn best_chain_locked_height(&self) -> Result<u32, CoreError> {
        self.get_best_chain_lock().map(|lock| lock.block_height)
    }

    fn quorum_list(&self, height: u32) -> Result<ExtendedQuorumListResult, CoreError> {
        self.get_quorum_listextended_reversed(Some(height))
    }

    fn protx_diff(&self, base: u32, block: u32) -> Result<MasternodeListDiff, CoreError> {
        self.get_protx_listdiff(base, block)
    }
}

struct Pending<K, T> {
    key: K,
    result: Receiver<Result<T, CoreError>>,
}

/// Holds one in-flight speculative fetch of each kind.
pub struct CorePrefetcher<S: PrefetchSource = Client> {
    source: Arc<S>,
    /// Best chain-locked height Core last reported. Guesses above it are not
    /// made. Refreshed only when a guess would exceed it.
    chain_locked_height: AtomicU32,
    quorum_list: Mutex<Option<Pending<u32, ExtendedQuorumListResult>>>,
    protx_diff: Mutex<Option<Pending<(u32, u32), MasternodeListDiff>>>,
}

impl<S: PrefetchSource> std::fmt::Debug for CorePrefetcher<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CorePrefetcher")
    }
}

impl CorePrefetcher {
    /// Opens a second connection to Core for speculative calls. Returns `None`
    /// if it cannot be opened; prefetching is an optimisation, and a node that
    /// cannot open it should still run.
    pub fn new(url: &str, username: String, password: String) -> Option<Self> {
        let client = Client::new(url, Auth::UserPass(username, password)).ok()?;
        Some(Self::with_source(client))
    }
}

impl<S: PrefetchSource> CorePrefetcher<S> {
    pub(crate) fn with_source(source: S) -> Self {
        CorePrefetcher {
            source: Arc::new(source),
            chain_locked_height: AtomicU32::new(0),
            quorum_list: Mutex::new(None),
            protx_diff: Mutex::new(None),
        }
    }

    /// True when `height` is known to be chain-locked. Asks Core once when the
    /// cached chain lock is behind; a failed refresh just means no guess now.
    fn is_chain_locked(&self, height: u32) -> bool {
        if height <= self.chain_locked_height.load(Ordering::Relaxed) {
            return true;
        }
        match self.source.best_chain_locked_height() {
            Ok(best) => {
                self.chain_locked_height.store(best, Ordering::Relaxed);
                height <= best
            }
            Err(_) => false,
        }
    }

    /// Takes the speculative quorum list for `height`, if one was started and
    /// succeeded. Blocks until the in-flight call finishes; only a Core stall
    /// can make that wait long, and a stall would hold up the real call too.
    pub fn take_quorum_list(&self, height: u32) -> Option<ExtendedQuorumListResult> {
        let pending = self.quorum_list.lock().ok()?.take()?;
        if pending.key != height {
            return None;
        }
        pending.result.recv().ok()?.ok()
    }

    /// Starts fetching the quorum list for `height` in the background, if that
    /// height is chain-locked.
    pub fn start_quorum_list(&self, height: u32) {
        if !self.is_chain_locked(height) {
            return;
        }
        let Ok(mut slot) = self.quorum_list.lock() else {
            return;
        };
        let (tx, rx) = sync_channel(1);
        let source = Arc::clone(&self.source);
        std::thread::spawn(move || {
            let _ = tx.send(source.quorum_list(height));
        });
        *slot = Some(Pending {
            key: height,
            result: rx,
        });
    }

    /// Takes the speculative masternode list diff for `base -> block`, if one
    /// was started and succeeded.
    pub fn take_protx_diff(&self, base: u32, block: u32) -> Option<MasternodeListDiff> {
        let pending = self.protx_diff.lock().ok()?.take()?;
        if pending.key != (base, block) {
            return None;
        }
        pending.result.recv().ok()?.ok()
    }

    /// Starts fetching the masternode list diff `base -> block` in the
    /// background, if `block` is chain-locked.
    pub fn start_protx_diff(&self, base: u32, block: u32) {
        if !self.is_chain_locked(block) {
            return;
        }
        let Ok(mut slot) = self.protx_diff.lock() else {
            return;
        };
        let (tx, rx) = sync_channel(1);
        let source = Arc::clone(&self.source);
        std::thread::spawn(move || {
            let _ = tx.send(source.protx_diff(base, block));
        });
        *slot = Some(Pending {
            key: (base, block),
            result: rx,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::dashcore_rpc::jsonrpc;
    use std::sync::atomic::AtomicUsize;

    /// A Core that is chain-locked up to `chain_lock` and answers every fetch,
    /// counting the calls it receives.
    struct FakeCore {
        chain_lock: AtomicU32,
        chain_lock_calls: AtomicUsize,
        quorum_list_calls: AtomicUsize,
        protx_diff_calls: AtomicUsize,
    }

    impl FakeCore {
        fn locked_up_to(height: u32) -> Self {
            FakeCore {
                chain_lock: AtomicU32::new(height),
                chain_lock_calls: AtomicUsize::new(0),
                quorum_list_calls: AtomicUsize::new(0),
                protx_diff_calls: AtomicUsize::new(0),
            }
        }
    }

    fn not_found() -> CoreError {
        CoreError::JsonRpc(jsonrpc::error::Error::Rpc(jsonrpc::error::RpcError {
            code: -8,
            message: "Block height out of range".into(),
            data: None,
        }))
    }

    impl PrefetchSource for FakeCore {
        fn best_chain_locked_height(&self) -> Result<u32, CoreError> {
            self.chain_lock_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.chain_lock.load(Ordering::SeqCst))
        }

        fn quorum_list(&self, height: u32) -> Result<ExtendedQuorumListResult, CoreError> {
            self.quorum_list_calls.fetch_add(1, Ordering::SeqCst);
            if height > self.chain_lock.load(Ordering::SeqCst) {
                return Err(not_found());
            }
            Ok(ExtendedQuorumListResult {
                quorums_by_type: Default::default(),
            })
        }

        fn protx_diff(&self, base: u32, block: u32) -> Result<MasternodeListDiff, CoreError> {
            self.protx_diff_calls.fetch_add(1, Ordering::SeqCst);
            if block > self.chain_lock.load(Ordering::SeqCst) {
                return Err(not_found());
            }
            Ok(MasternodeListDiff {
                base_height: base,
                block_height: block,
                added_mns: vec![],
                removed_mns: vec![],
                updated_mns: vec![],
            })
        }
    }

    #[test]
    fn a_guess_below_the_chain_lock_is_fetched_and_taken() {
        let prefetcher = CorePrefetcher::with_source(FakeCore::locked_up_to(100));

        prefetcher.start_quorum_list(50);
        let list = prefetcher.take_quorum_list(50);

        assert!(list.is_some());
        assert_eq!(
            prefetcher.source.quorum_list_calls.load(Ordering::SeqCst),
            1
        );
    }

    #[test]
    fn a_guess_above_the_chain_lock_is_not_made() {
        let prefetcher = CorePrefetcher::with_source(FakeCore::locked_up_to(100));

        prefetcher.start_quorum_list(101);
        prefetcher.start_protx_diff(100, 101);

        assert!(prefetcher.take_quorum_list(101).is_none());
        assert!(prefetcher.take_protx_diff(100, 101).is_none());
        assert_eq!(
            prefetcher.source.quorum_list_calls.load(Ordering::SeqCst),
            0
        );
        assert_eq!(prefetcher.source.protx_diff_calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn the_chain_lock_is_only_refreshed_when_a_guess_would_pass_it() {
        let core = FakeCore::locked_up_to(100);
        let prefetcher = CorePrefetcher::with_source(core);

        // First guess: cache is 0, so one refresh; then 1..=100 are covered.
        for height in 1..=100 {
            prefetcher.start_protx_diff(height - 1, height);
            assert!(prefetcher.take_protx_diff(height - 1, height).is_some());
        }
        assert_eq!(prefetcher.source.chain_lock_calls.load(Ordering::SeqCst), 1);

        // A guess past the lock refreshes once more and is then declined.
        prefetcher.start_protx_diff(100, 101);
        assert!(prefetcher.take_protx_diff(100, 101).is_none());
        assert_eq!(prefetcher.source.chain_lock_calls.load(Ordering::SeqCst), 2);

        // Core moves on; the next guess refreshes and succeeds.
        prefetcher.source.chain_lock.store(101, Ordering::SeqCst);
        prefetcher.start_protx_diff(100, 101);
        assert!(prefetcher.take_protx_diff(100, 101).is_some());
        assert_eq!(prefetcher.source.chain_lock_calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn a_take_for_a_different_key_discards_the_guess() {
        let prefetcher = CorePrefetcher::with_source(FakeCore::locked_up_to(100));

        prefetcher.start_quorum_list(50);
        assert!(prefetcher.take_quorum_list(51).is_none());
        // The slot was consumed by the mismatched take.
        assert!(prefetcher.take_quorum_list(50).is_none());

        prefetcher.start_protx_diff(49, 50);
        assert!(prefetcher.take_protx_diff(48, 50).is_none());
    }
}
