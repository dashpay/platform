//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::wallet::platform_wallet::WalletId;

/// Storage backend for [`PlatformWalletChangeSet`] deltas.
///
/// The persister persists what the changeset carries — nothing more,
/// nothing less. See the **Scope** section below for the exact
/// boundary.
///
/// Changesets flow through a two-phase pipeline:
///
/// 1. **`store`** — merge a delta into per-wallet state. Implementations
///    MAY defer I/O until [`flush`](Self::flush) or MAY flush inline;
///    callers must not assume `store` is free of I/O. See the
///    **Nuance** section below.
/// 2. **`flush`** — ensure all buffered deltas for the given `wallet_id`
///    are durable, then clear that wallet's buffer.
///
/// The trait uses `&self` with a `wallet_id` parameter so a single persister
/// instance can be shared across all wallets in a [`PlatformWalletManager`].
/// Implementations are responsible for internal synchronization (e.g.
/// `Mutex` / `RwLock` around staged changeset buffers).
///
/// # Scope
///
/// ## Inside scope — must be persisted via `store` / `flush`
///
/// - **Wallet-level core state**: chain height, per-account address-pool
///   watermarks (`highest_used`), UTXO set (inserts, spends, IS-lock flags),
///   per-account transaction records, and account pool state.
/// - **Asset locks**: all tracked asset-lock transactions with their funding
///   type, proof, and chain-lock state (`AssetLockChangeSet`).
/// - **Identity-level state** (`IdentityEntry` inside `IdentityChangeSet`):
///   wallet_id, wallet_index, DPNS usernames, top-up history, lifecycle
///   status, key storage, DashPay profile, and DashPay payment history.
/// - **Contact state** (`ContactChangeSet`): sent contact requests, incoming
///   (received) contact requests, and established (mutually-accepted) contacts.
///
/// ## Outside scope — persisted through other mechanisms
///
/// - **Raw `identity.data` BLOB**: the bincoded `QualifiedIdentity` record
///   (an evo-tool wrapper type around `dpp::Identity`) is currently written
///   directly by `Database::insert_local_qualified_identity` and
///   `Database::update_local_qualified_identity`, called from backend tasks.
///   Moving this blob into the persister is planned as a future commit
///   (evo-tool task #130 / Phase 9c). Until then, the persister does not
///   write or read the `identity.data` column.
/// - **Platform addresses** and **token balances**: these are dropped on
///   flush; backend tasks own their persistence.
///
/// ## Nuance on `store` and I/O
///
/// The `store` method is documented as "buffer for later writing (cheap, no
/// I/O)", but implementations are free to flush inline. In particular,
/// `SqliteWalletPersister` (the canonical evo-tool implementation) flushes
/// on every `store` call — there is no deferred batch window. Callers must
/// not assume that `store` is free of I/O; treat any `store` → `flush`
/// sequence as potentially performing I/O at either point. If a caller needs
/// to guarantee a batch flush, it should call `flush` explicitly after all
/// `store` calls and treat `store` as a best-effort buffer hint.
pub trait PlatformWalletPersistence: Send + Sync {
    /// Buffer a changeset for later persistence.
    ///
    /// Implementations should merge into an internal per-wallet accumulator so
    /// that a single [`flush`](Self::flush) writes the combined delta.
    ///
    /// Returns an error if the internal accumulator cannot be accessed
    /// (e.g. mutex poisoning). Callers that use fire-and-forget
    /// semantics should log the error rather than propagating.
    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Write all buffered changesets atomically for the given wallet, then
    /// clear that wallet's buffer.
    fn flush(&self, wallet_id: WalletId) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Load the full wallet state from storage.
    ///
    /// Returns a single [`PlatformWalletChangeSet`] representing the full
    /// stored state (equivalent to merging all previously persisted deltas).
    fn load(
        &self,
        wallet_id: WalletId,
    ) -> Result<PlatformWalletChangeSet, Box<dyn std::error::Error + Send + Sync>>;
}
