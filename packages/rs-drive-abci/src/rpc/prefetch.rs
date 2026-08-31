//! Speculative fetching of the two Core RPC responses a block needs when the
//! core chain-locked height advances.
//!
//! Replaying mainnet history, roughly every other Platform block advances the
//! core height by one, and each of those blocks blocks on `protx listdiff` and
//! `quorum listextended` in turn — about a millisecond of the seven a block
//! costs. The heights are consecutive, so the answer for the next one can be
//! fetched while the current block is still executing.
//!
//! Two things keep this from misbehaving at the tip, where the next core block
//! does not exist yet: the speculative call runs on its own connection, so it
//! never delays a real one, and a failed guess backs the prefetcher off for a
//! while instead of asking Core for a block it does not have on every block.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{sync_channel, Receiver};
use std::sync::{Arc, Mutex};

use dpp::dashcore_rpc::dashcore_rpc_json::{ExtendedQuorumListResult, MasternodeListDiff};
use dpp::dashcore_rpc::{Auth, Client, Error as CoreError, RpcApi};

/// How many calls to skip after a speculative fetch fails. A failure means the
/// guessed height is not on Core's chain yet, which is the steady state at the
/// tip, and asking again on the next block would just repeat the error.
const BACKOFF_CALLS: u32 = 32;

struct Pending<K, T> {
    key: K,
    result: Receiver<Result<T, CoreError>>,
}

/// Holds one in-flight speculative fetch of each kind.
pub struct CorePrefetcher {
    client: Arc<Client>,
    quorum_list: Mutex<Option<Pending<u32, ExtendedQuorumListResult>>>,
    protx_diff: Mutex<Option<Pending<(u32, u32), MasternodeListDiff>>>,
    /// Calls left to skip before speculating again after a failure.
    backoff: AtomicU32,
}

impl std::fmt::Debug for CorePrefetcher {
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
        Some(CorePrefetcher {
            client: Arc::new(client),
            quorum_list: Mutex::new(None),
            protx_diff: Mutex::new(None),
            backoff: AtomicU32::new(0),
        })
    }

    fn may_speculate(&self) -> bool {
        let left = self.backoff.load(Ordering::Relaxed);
        if left == 0 {
            return true;
        }
        self.backoff.store(left - 1, Ordering::Relaxed);
        false
    }

    fn note_failure(&self) {
        self.backoff.store(BACKOFF_CALLS, Ordering::Relaxed);
    }

    /// Takes the speculative quorum list for `height`, if one was started and
    /// succeeded. Blocks until the in-flight call finishes.
    pub fn take_quorum_list(&self, height: u32) -> Option<ExtendedQuorumListResult> {
        let pending = self.quorum_list.lock().ok()?.take()?;
        if pending.key != height {
            return None;
        }
        match pending.result.recv().ok()? {
            Ok(list) => Some(list),
            Err(_) => {
                self.note_failure();
                None
            }
        }
    }

    /// Starts fetching the quorum list for `height` in the background.
    pub fn start_quorum_list(&self, height: u32) {
        if !self.may_speculate() {
            return;
        }
        let Ok(mut slot) = self.quorum_list.lock() else {
            return;
        };
        let (tx, rx) = sync_channel(1);
        let client = Arc::clone(&self.client);
        std::thread::spawn(move || {
            let _ = tx.send(client.get_quorum_listextended_reversed(Some(height)));
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
        match pending.result.recv().ok()? {
            Ok(diff) => Some(diff),
            Err(_) => {
                self.note_failure();
                None
            }
        }
    }

    /// Starts fetching the masternode list diff `base -> block` in the background.
    pub fn start_protx_diff(&self, base: u32, block: u32) {
        if !self.may_speculate() {
            return;
        }
        let Ok(mut slot) = self.protx_diff.lock() else {
            return;
        };
        let (tx, rx) = sync_channel(1);
        let client = Arc::clone(&self.client);
        std::thread::spawn(move || {
            let _ = tx.send(client.get_protx_listdiff(base, block));
        });
        *slot = Some(Pending {
            key: (base, block),
            result: rx,
        });
    }
}
