//! Persistence traits for wallet storage backends.
//!
//! Implementors choose their own storage engine (SQLite, file, memory, remote).
//! The traits guarantee that deltas are persisted atomically.

use std::error::Error as StdError;

use crate::changeset::changeset::PlatformWalletChangeSet;
use crate::changeset::client_start_state::ClientStartState;
use crate::wallet::platform_wallet::WalletId;
use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;

/// Retry classification for [`PersistenceError::Backend`].
///
/// The kind carries the persister's `is_transient()` contract across
/// the trait boundary so consumers can decide whether to retry, undo
/// in-memory state, or surface the failure to the user without
/// guessing from a string message.
///
/// The enum is intentionally NOT `#[non_exhaustive]`: adding a new
/// kind MUST force every consumer match to update explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PersistenceErrorKind {
    /// The persister reports the write was not committed and the
    /// buffered state is preserved (e.g. `SQLITE_BUSY`, `SQLITE_FULL`,
    /// `SQLITE_IOERR`, `SQLITE_NOMEM`). Callers MAY retry with
    /// exponential backoff.
    Transient,
    /// The persister reports an unrecoverable failure (schema
    /// corruption, logic bug, I/O error not covered by the transient
    /// class). Callers MUST NOT retry — the buffered changeset is
    /// gone and the same call will keep failing.
    Fatal,
    /// SQL constraint / foreign-key / integrity violation. Distinct
    /// from `Fatal` so callers can distinguish "your data is wrong"
    /// (caller bug) from "the storage engine is unhappy" (operator /
    /// infrastructure problem). Treated as fatal for retry purposes.
    Constraint,
}

/// Errors returned by a [`PlatformWalletPersistence`] backend.
///
/// Concrete (non-`Box<dyn Error>`) so callers and downstream
/// traits can compose the result types without erasing the
/// error's shape. Backends that don't fit cleanly into
/// [`Self::LockPoisoned`] route their native error through
/// [`Self::backend_with_kind`] (or [`Self::backend`] when the kind
/// isn't known) into [`Self::Backend`].
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// An internal synchronization primitive is poisoned (a
    /// previous holder panicked while mutating state). Most
    /// backends are unable to recover from this and should
    /// treat it as fatal.
    #[error("persister lock poisoned")]
    LockPoisoned,

    /// Error bubbled up from the underlying storage engine
    /// (SQLite, file I/O, FFI callback, etc.).
    ///
    /// `kind` carries the retry classification — see
    /// [`PersistenceErrorKind`]. `source` is a boxed typed error so
    /// callers that need finer detail can downcast (the canonical
    /// SQLite backend boxes `WalletStorageError`, which preserves the
    /// full typed source chain).
    #[error("persistence backend error ({kind:?}): {source}")]
    Backend {
        kind: PersistenceErrorKind,
        source: Box<dyn StdError + Send + Sync>,
    },
}

impl PersistenceError {
    /// Construct a [`Self::Backend`] from any boxable error,
    /// classified as [`PersistenceErrorKind::Fatal`].
    ///
    /// Use this when the caller does not (or cannot) classify the
    /// kind. Defaulting to `Fatal` is the conservative choice: a
    /// misclassification reads as "do not retry" rather than
    /// spuriously retrying a permanent failure.
    pub fn backend<E>(source: E) -> Self
    where
        E: Into<Box<dyn StdError + Send + Sync>>,
    {
        Self::Backend {
            kind: PersistenceErrorKind::Fatal,
            source: source.into(),
        }
    }

    /// Construct a [`Self::Backend`] with an explicit kind. Use this
    /// at the persister boundary where the kind is known (e.g.
    /// `From<WalletStorageError>` checks `is_transient()` and the
    /// constraint codes before calling this).
    pub fn backend_with_kind<E>(kind: PersistenceErrorKind, source: E) -> Self
    where
        E: Into<Box<dyn StdError + Send + Sync>>,
    {
        Self::Backend {
            kind,
            source: source.into(),
        }
    }

    /// `true` if the error is a `Backend` whose kind is
    /// [`PersistenceErrorKind::Transient`]. `LockPoisoned`, `Fatal`,
    /// and `Constraint` all read as non-transient.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Backend {
                kind: PersistenceErrorKind::Transient,
                ..
            }
        )
    }

    /// Retry-policy classification for the error.
    ///
    /// Returns `None` for [`Self::LockPoisoned`] (which is its own
    /// trait-level variant) and `Some(kind)` for [`Self::Backend`].
    /// Callers that always need a kind should treat `None` as
    /// [`PersistenceErrorKind::Fatal`].
    pub fn kind(&self) -> Option<PersistenceErrorKind> {
        match self {
            Self::LockPoisoned => None,
            Self::Backend { kind, .. } => Some(*kind),
        }
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
    /// Whether stored state survives a process restart once `store` + `flush`
    /// return `Ok`.
    ///
    /// Defaults to `true` — the contract every real backend (SQLite, the FFI
    /// SwiftData bridge) meets. Backends that only buffer in memory or drop
    /// writes (e.g. [`NoPlatformPersistence`](crate::wallet::persister::NoPlatformPersistence))
    /// MUST override this to return `false`.
    ///
    /// Security-sensitive flows gate on this. Creating a DashPay invitation
    /// exports a one-time bearer voucher key derived from a persisted funding
    /// index; on a backend that cannot guarantee the index survives a restart,
    /// the same key could be re-derived and re-exported after a relaunch,
    /// letting the holder of an earlier link consume a later voucher. Such
    /// flows refuse to run on a non-durable backend rather than silently
    /// producing a reusable bearer secret.
    fn persists_durably(&self) -> bool {
        true
    }

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
    /// Implementations classify failures via
    /// [`PersistenceErrorKind`] on the returned
    /// [`PersistenceError::Backend`] so callers can drive retry policy
    /// off [`PersistenceError::is_transient`]:
    ///
    /// - **[`PersistenceErrorKind::Transient`]** — for the canonical
    ///   SQLite backend that's `SQLITE_BUSY` / `SQLITE_LOCKED` plus the
    ///   I/O-class codes `SQLITE_FULL` / `SQLITE_IOERR` /
    ///   `SQLITE_NOMEM`: the buffered changeset is
    ///   preserved (re-merged via the buffer's `restore` path so any
    ///   `store` that landed during the failed flush wins on LWW
    ///   fields), and the caller MAY retry with exponential backoff.
    /// - **[`PersistenceErrorKind::Constraint`]** — SQL
    ///   constraint / FK / integrity violation. Caller bug; the data
    ///   is rejected by the schema. MUST NOT retry without changing
    ///   the data.
    /// - **[`PersistenceErrorKind::Fatal`]** — everything else
    ///   (schema corruption, logic bugs, I/O outside the transient
    ///   class): the buffer is dropped, the staged changeset is gone,
    ///   and the backend logs a structured `tracing::error!`. The
    ///   caller MUST NOT retry — the data is not recoverable through
    ///   this trait.
    ///
    /// [`PersistenceError::LockPoisoned`] is fatal but distinguished
    /// at the variant level so callers can pattern-match on it.
    ///
    // TODO: wallet-less / global objects (the `WalletId::default()` /
    // `[0u8; 32]` sentinel scope for parentless or global metadata) are
    // not yet expressible through `flush`. Hosts that previously called
    // the now-removed `commit_writes` should call `flush` per wallet
    // instead; a sentinel-scope flush path is still to be designed.
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

    // TODO: `list_wallets` and `delete_wallet` are deferred contract
    // candidates. They live as inherent methods on the SQLite backend
    // today; they may return to this trait once a cross-backend contract
    // (consistent error/report semantics across SQLite, file, and FFI
    // backends) is agreed.
}
