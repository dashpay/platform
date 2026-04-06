//! Shared SPV sync state — atomics accessible without holding the adapter's RwLock.
//!
//! During `process_block()` the `SpvWalletAdapter` holds a write lock. If these
//! atomics lived behind that lock, `SpvRuntime::synced_height()` would return 0
//! whenever a block is being processed. By extracting them into a shared
//! `Arc<SpvSyncState>`, both the runtime and the adapter can access them
//! concurrently without contention.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Shared SPV sync progress atomics.
///
/// Held by both `SpvRuntime` (for public status queries) and
/// `SpvWalletAdapter` (for updates during block/mempool processing).
/// No lock needed — atomics are self-synchronizing.
pub(crate) struct SpvSyncState {
    pub synced_height: AtomicU32,
    pub filter_committed_height: AtomicU32,
    pub monitor_revision: AtomicU64,
}

impl SpvSyncState {
    pub fn new() -> Self {
        Self {
            synced_height: AtomicU32::new(0),
            filter_committed_height: AtomicU32::new(0),
            monitor_revision: AtomicU64::new(0),
        }
    }

    pub fn synced_height(&self) -> u32 {
        self.synced_height.load(Ordering::Relaxed)
    }

    pub fn update_synced_height(&self, height: u32) {
        self.synced_height.store(height, Ordering::Relaxed);
    }

    pub fn filter_committed_height(&self) -> u32 {
        self.filter_committed_height.load(Ordering::Relaxed)
    }

    pub fn update_filter_committed_height(&self, height: u32) {
        self.filter_committed_height.store(height, Ordering::Relaxed);
    }

    pub fn monitor_revision(&self) -> u64 {
        self.monitor_revision.load(Ordering::Relaxed)
    }

    pub fn bump_monitor_revision(&self) {
        self.monitor_revision.fetch_add(1, Ordering::Relaxed);
    }
}
