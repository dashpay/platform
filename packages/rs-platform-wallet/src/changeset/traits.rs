//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::client_start_state::ClientStartState;
use crate::wallet::platform_wallet::WalletId;
use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;

/// Errors returned by a [`PlatformWalletPersistence`] backend.
///
/// Concrete (non-`Box<dyn Error>`) so callers and downstream
/// traits can compose the result types without erasing the
/// error's shape. Backends that don't fit cleanly into
/// [`Self::LockPoisoned`] render their native error via
/// [`Self::backend`] into [`Self::Backend`].
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// An internal synchronization primitive is poisoned (a
    /// previous holder panicked while mutating state). Most
    /// backends are unable to recover from this and should
    /// treat it as fatal.
    #[error("persister lock poisoned")]
    LockPoisoned,

    /// Error bubbled up from the underlying storage engine
    /// (SQLite, file I/O, FFI callback, etc.). Carries the
    /// backend's error message; the original error type is
    /// intentionally erased so the trait stays object-safe
    /// without generic error parameters.
    #[error("persistence backend error: {0}")]
    Backend(String),
}

impl PersistenceError {
    /// Convenience constructor that stringifies any
    /// `Display` error into [`PersistenceError::Backend`].
    pub fn backend(err: impl std::fmt::Display) -> Self {
        Self::Backend(err.to_string())
    }
}

// Ergonomic conversions so backends can `.into()` a message without
// spelling out the enum variant. The common pattern in FFI-style
// backends is `Err(format!("...").into())`; the `From<String>` impl
// keeps that terse while routing into the typed error.
impl From<String> for PersistenceError {
    fn from(msg: String) -> Self {
        Self::Backend(msg)
    }
}

impl From<&str> for PersistenceError {
    fn from(msg: &str) -> Self {
        Self::Backend(msg.to_string())
    }
}

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
    ) -> Result<(), PersistenceError>;

    /// Write all buffered changesets atomically for the given wallet, then
    /// clear that wallet's buffer.
    ///
    /// # Errors
    ///
    /// Implementations classify failures along a two-axis contract:
    ///
    /// - **Transient** (`PersistenceError::backend(..)` whose source
    ///   carries `is_transient() == true` — for the canonical SQLite
    ///   backend that's `SQLITE_BUSY` / `SQLITE_LOCKED`, and as of
    ///   ATOM-008 also the I/O-class codes `SQLITE_FULL` /
    ///   `SQLITE_IOERR` / `SQLITE_NOMEM`): the buffered changeset is
    ///   preserved (re-merged via the buffer's `restore` path so any
    ///   `store` that landed during the failed flush wins on LWW
    ///   fields), and the caller MAY retry with exponential backoff.
    /// - **Fatal** (everything else — schema corruption, logic bugs,
    ///   integrity violations): the buffer is dropped, the staged
    ///   changeset is gone, and the backend logs a structured
    ///   `tracing::error!`. The caller MUST NOT retry — the data is
    ///   not recoverable through this trait.
    ///
    /// [`PersistenceError::LockPoisoned`] is fatal but distinguished
    /// at the variant level so callers can pattern-match on it.
    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError>;

    /// Load the full client state from storage.
    ///
    /// Returns a [`ClientStartState`] — a ready-to-boot snapshot covering
    /// every wallet the persister knows about. It mirrors
    /// [`PlatformWalletChangeSet`] for most sub-areas, but platform-address
    /// state comes back as
    /// [`PlatformAddressSyncStartState`](crate::changeset::PlatformAddressSyncStartState)
    /// so the caller can hand it straight to
    /// `PlatformPaymentAddressProvider::from_persisted` instead of
    /// replaying an accumulated changeset.
    ///
    /// Called once at startup — this is a whole-client operation, not a
    /// per-wallet one, because `ClientStartState::platform_addresses` is
    /// already keyed by wallet id and the sub-changesets carry their own
    /// wallet attribution where needed.
    fn load(&self) -> Result<ClientStartState, PersistenceError>;

    /// Look up a single core transaction record by `txid` for `wallet_id`.
    ///
    /// Used by the asset-lock proof flow to recover records that the
    /// in-memory `transactions()` map has evicted. Upstream's
    /// `keep-finalized-transactions` Cargo feature is OFF by default —
    /// chainlocked records are dropped from the in-memory map and only
    /// their txids are retained in `finalized_txids` for dedup. The
    /// chain-lock height / block hash that an asset-lock proof needs is
    /// no longer reachable through the wallet-info API, but the
    /// persister received the full record on the last `store` call
    /// before eviction, so it can answer this lookup.
    ///
    /// The default implementation returns `Ok(None)` — backwards
    /// compatible for persisters that don't index records by txid (e.g.
    /// [`NoPlatformPersistence`]). The asset-lock proof flow's hot path
    /// (mempool / `InBlock` window) hits the in-memory map first, so a
    /// `None` response from this method only matters for the rare race
    /// where the first lookup happens after the chainlock-eviction
    /// window. Persisters whose backing store keys records by txid
    /// (`SqliteWalletPersister`, the SwiftData iOS persister) should
    /// override.
    ///
    /// **Field contract.** Implementations are only required to
    /// populate `txid` and `context` (with the `BlockInfo` inside
    /// `InChainLockedBlock` / `InBlock` carrying real height + block
    /// hash + timestamp). Other fields (`transaction`, `input_details`,
    /// `output_details`, `account_type`, `transaction_type`,
    /// `direction`, `net_amount`, `fee`, `label`) MAY be returned as
    /// best-effort placeholders and MUST NOT be relied upon by callers.
    /// The current consumer — the asset-lock proof flow — only reads
    /// `context` and `height()` (which is
    /// `context.block_info().map(|b| b.height)`). FFI-backed
    /// implementations (e.g. the SwiftData iOS persister) take
    /// advantage of this contract by emitting a synthetic record with a
    /// placeholder transaction body, since reconstructing the full
    /// `Transaction` over the C ABI is not free and isn't needed.
    fn get_core_tx_record(
        &self,
        _wallet_id: WalletId,
        _txid: &Txid,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        Ok(None)
    }
}
