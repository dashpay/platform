//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use crate::persistence::changeset::WalletChangeSet;

/// Synchronous storage backend for wallet state.
///
/// Every call to [`persist`](WalletPersistence::persist) must be atomic:
/// either all sub-changesets are stored or none are. Implementations should
/// use database transactions, atomic file writes, or equivalent mechanisms.
pub trait WalletPersistence {
    /// Error type returned by this backend.
    type Error: std::error::Error;

    /// Load the aggregated state from storage.
    ///
    /// Returns a single [`WalletChangeSet`] representing the full stored state
    /// (equivalent to merging all previously persisted deltas).
    fn initialize(&mut self) -> Result<WalletChangeSet, Self::Error>;

    /// Persist a delta atomically.
    fn persist(&mut self, changeset: &WalletChangeSet) -> Result<(), Self::Error>;
}

/// Async storage backend for wallet state.
///
/// Same contract as [`WalletPersistence`] but for async runtimes.
#[async_trait::async_trait]
pub trait AsyncWalletPersistence: Send + Sync {
    /// Error type returned by this backend.
    type Error: std::error::Error + Send + Sync;

    /// Load the aggregated state from storage.
    async fn initialize(&mut self) -> Result<WalletChangeSet, Self::Error>;

    /// Persist a delta atomically.
    async fn persist(&mut self, changeset: &WalletChangeSet) -> Result<(), Self::Error>;
}
