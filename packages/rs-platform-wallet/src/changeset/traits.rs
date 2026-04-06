//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::wallet::platform_wallet::WalletId;

/// Storage backend for platform wallet state.
///
/// Changesets flow through a two-phase pipeline:
///
/// 1. **`queue`** — buffer a delta for later writing (cheap, no I/O).
/// 2. **`flush`** — write all queued deltas atomically.
///
/// This decouples the hot path (SPV block processing, mempool updates) from
/// disk I/O, letting callers batch many small deltas before committing.
///
/// The trait uses `&self` with a `wallet_id` parameter so a single persister
/// instance can be shared across all wallets in a [`PlatformWalletManager`].
/// Implementations are responsible for internal synchronization (e.g.
/// `Mutex` / `RwLock` around staged changeset buffers).
pub trait PlatformWalletPersistence: Send + Sync {
    /// Buffer a changeset for later persistence.
    ///
    /// Implementations should merge into an internal per-wallet accumulator so
    /// that a single [`flush`](Self::flush) writes the combined delta.
    fn queue(&self, wallet_id: WalletId, changeset: PlatformWalletChangeSet);

    /// Write all queued changesets atomically for the given wallet, then clear
    /// that wallet's queue.
    fn flush(&self, wallet_id: WalletId) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Load the aggregated state from storage for the given wallet.
    ///
    /// Returns a single [`PlatformWalletChangeSet`] representing the full
    /// stored state (equivalent to merging all previously persisted deltas).
    fn initialize(
        &self,
        wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>>;
}
