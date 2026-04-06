//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use crate::changeset::changeset::PlatformWalletChangeSet;

/// Storage backend for platform wallet state.
///
/// Changesets flow through a two-phase pipeline:
///
/// 1. **`queue`** — buffer a delta for later writing (cheap, no I/O).
/// 2. **`flush`** — write all queued deltas atomically.
///
/// This decouples the hot path (SPV block processing, mempool updates) from
/// disk I/O, letting callers batch many small deltas before committing.
pub trait PlatformWalletPersistence: Send + Sync {
    /// Buffer a changeset for later persistence.
    ///
    /// Implementations should merge into an internal accumulator so that a
    /// single [`flush`](Self::flush) writes the combined delta.
    fn queue(&mut self, changeset: PlatformWalletChangeSet);

    /// Write all queued changesets atomically, then clear the queue.
    fn flush(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Load the aggregated state from storage.
    ///
    /// Returns a single [`PlatformWalletChangeSet`] representing the full
    /// stored state (equivalent to merging all previously persisted deltas).
    fn initialize(
        &mut self,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>>;
}
