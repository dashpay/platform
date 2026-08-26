//! File-backed `ShieldedStore` impl.
//!
//! The Orchard commitment tree is shared across every subwallet
//! that decrypts notes against the same network — the on-chain
//! commitment stream is identical for every consumer. This store
//! therefore persists the tree to a SQLite file (via
//! [`ClientPersistentCommitmentTree`]) and keeps per-subwallet
//! decrypted notes / nullifier bookkeeping in memory, scoped by
//! [`SubwalletId`]. Notes are rediscovered on cold start by
//! re-running [`ShieldedWallet::sync_notes`] against the cached
//! tree (or, when the host persister is wired up, restored from
//! SwiftData before sync runs).

use std::collections::BTreeMap;
use std::error::Error as StdError;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};
use rusqlite::{Connection, OptionalExtension};

use super::store::{
    AdmissionToken, ClaimKeyReservation, ClaimKeyReservationOutcome, PendingRedrive, ShieldedNote,
    ShieldedOutgoingNote, ShieldedStore, StalePendingSpend, SubwalletId, SubwalletState,
};
use crate::wallet::platform_wallet::WalletId;

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub struct FileShieldedStoreError(pub String);

/// One raw `shielded_pending_spends` row as read from SQLite, before the key
/// widths and nullifier blob are validated: `(wallet_id, account_index,
/// activity_id, anchor, nullifiers, st_bytes, attempts, identity_index)`.
type PendingSpendRow = (
    Vec<u8>,
    u32,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    Vec<u8>,
    u32,
    Option<u32>,
);

impl fmt::Display for FileShieldedStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl StdError for FileShieldedStoreError {}

/// File-backed shielded store: SQLite-persisted commitment tree
/// plus in-memory per-subwallet decrypted notes / nullifier
/// bookkeeping.
pub struct FileBackedShieldedStore {
    /// SQLite-backed commitment tree. Wrapped in a `Mutex` because
    /// the underlying SQLite store is not `Sync`; the
    /// [`ShieldedStore`] trait requires `Send + Sync`. Outer
    /// concurrency is still serialized through `ShieldedWallet`'s
    /// `RwLock<S>`; this inner mutex is just a `Sync`-restoring
    /// shim and is uncontended in practice.
    tree: Mutex<ClientPersistentCommitmentTree>,
    /// Backing SQLite path, retained so [`reset_commitment_tree`]
    /// can wipe the on-disk tree tables and rebuild a fresh
    /// `ClientPersistentCommitmentTree` over the same file. The
    /// wrapper takes its `Connection` by value and exposes no
    /// public truncate, so a full reset reopens the tree rather
    /// than mutating the live handle in place.
    ///
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    path: PathBuf,
    /// `max_checkpoints` passed at open time, retained so the
    /// rebuilt tree in [`reset_commitment_tree`] matches the
    /// original retention policy.
    ///
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    max_checkpoints: usize,
    /// Per-subwallet notes + sync state, keyed by `(wallet_id,
    /// account_index)`. Lazily populated on first use of an id.
    subwallets: BTreeMap<SubwalletId, SubwalletState>,
    /// Second connection on the same SQLite file, owning the
    /// `shielded_pending_spends` table (armed [`PendingRedrive`]
    /// records). Separate from `tree` because the commitment-tree
    /// wrapper takes its `Connection` by value; WAL mode makes the
    /// two-connection setup safe. `Mutex` for the same `Sync`-shim
    /// reason as `tree`.
    pending_conn: Mutex<rusqlite::Connection>,
    /// Test-only injection point for a failing [`ShieldedStore::purge_wallet`]
    /// — see [`fail_purge_wallet_for_tests`](Self::fail_purge_wallet_for_tests).
    #[cfg(test)]
    fail_purge_wallet: std::sync::atomic::AtomicBool,
}

impl FileBackedShieldedStore {
    /// Open or create a shielded store at `path`.
    ///
    /// SQLite is opened with **WAL journal + synchronous=NORMAL + temp_store=MEMORY**
    /// rather than the rusqlite defaults (DELETE + sync=FULL). Rationale: every
    /// `append_commitment` invocation runs an implicit one-statement transaction
    /// that, under DELETE+FULL, forces a fsync per cmx. On hosts where fsync is
    /// strictly honored (macOS Mac/simulator filesystems), that turns into the
    /// dominant cost of cold sync — a 1M-leaf tree build was ~6 min, vs ~17 s
    /// with the PRAGMAs below, per
    /// `packages/rs-platform-wallet/tests/shielded_tree_append_bench.rs`.
    ///
    /// `synchronous=NORMAL` retains crash-safety for the WAL (the WAL itself is
    /// fsync'd at checkpoint); we don't need `FULL` because no row in the
    /// commitment-tree SQLite is "user money" — every commitment is chain-side
    /// authenticated and can be rebuilt by re-running sync from a recorded
    /// `last_synced_note_index`. A torn WAL on power loss would at worst
    /// require resync from the last checkpoint, which is the same cost the
    /// host already accepts on a fresh install.
    pub fn open_path(
        path: impl AsRef<Path>,
        max_checkpoints: usize,
    ) -> Result<Self, FileShieldedStoreError> {
        let path = path.as_ref().to_path_buf();
        let conn = Self::open_tuned_connection(&path)?;
        let tree = ClientPersistentCommitmentTree::open(conn, max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("open commitment tree: {e}")))?;
        // `open_durable_connection`, NOT `open_tuned_connection`: this
        // connection owns the unreconstructable claim-recovery row, so it runs
        // at `synchronous=FULL` (#4313 review finding file_store.rs:107). The
        // tree connection above keeps NORMAL — see both doc comments.
        let mut pending_conn = Self::open_durable_connection(&path)?;
        pending_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS shielded_pending_spends (
                    wallet_id      BLOB    NOT NULL,
                    account_index  INTEGER NOT NULL,
                    activity_id    BLOB    NOT NULL,
                    anchor         BLOB    NOT NULL,
                    nullifiers     BLOB    NOT NULL,
                    st_bytes       BLOB    NOT NULL,
                    attempts       INTEGER NOT NULL DEFAULT 0,
                    identity_index INTEGER,
                    PRIMARY KEY (wallet_id, account_index, activity_id)
                )",
                [],
            )
            .map_err(|e| FileShieldedStoreError(format!("create pending_spends table: {e}")))?;
        Self::add_pending_spends_identity_index(&mut pending_conn)?;
        // Cross-instance / cross-PROCESS lifecycle admission (#4313). Lives in
        // the same SQLite file as the records it protects — that file is the
        // only thing two `FileBackedShieldedStore` instances (or two
        // processes) opened on the same path actually share, and SQLite's
        // one-writer-at-a-time rule is what makes the protocol's two entry
        // points totally ordered. See `store::LifecycleAdmission`.
        //
        // Deliberately NOT rehydrated into memory and deliberately not wiped
        // at open: rows are judged purely by `expires_at`, so a holder that
        // died leaves an entry that simply ages out, and a LIVE holder in
        // another process keeps its admission across our open.
        pending_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS shielded_lifecycle_admission (
                    token       BLOB    NOT NULL PRIMARY KEY,
                    destructive INTEGER NOT NULL,
                    wallet_id   BLOB,
                    expires_at  INTEGER NOT NULL
                )",
                [],
            )
            .map_err(|e| {
                FileShieldedStoreError(format!("create lifecycle_admission table: {e}"))
            })?;
        // One-time claim-key reservations (#4313 review finding cr-9d0e1a44).
        // The lifecycle admission above is per-WALLET and orders a claim
        // against a purge; it admits BOTH claims of the same invitation. This
        // table is per-INVITATION: the PRIMARY KEY is what turns
        // `INSERT ... ON CONFLICT DO NOTHING` into real mutual exclusion
        // between two coordinators — or two processes — on one file, which is
        // exactly where the coordinator's per-FVK mutex has no reach.
        //
        // Same lifetime rules as the admission table: rows are judged purely by
        // `expires_at` and never wiped at open, so a live holder in another
        // process keeps its reservation across our open while a dead one ages
        // out.
        pending_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS shielded_one_time_claim_reservation (
                    wallet_id        BLOB    NOT NULL,
                    claim_record_key BLOB    NOT NULL,
                    token            BLOB    NOT NULL,
                    expires_at       INTEGER NOT NULL,
                    PRIMARY KEY (wallet_id, claim_record_key)
                )",
                [],
            )
            .map_err(|e| {
                FileShieldedStoreError(format!("create one_time_claim_reservation table: {e}"))
            })?;
        let mut store = Self {
            tree: Mutex::new(tree),
            path,
            max_checkpoints,
            subwallets: BTreeMap::new(),
            pending_conn: Mutex::new(pending_conn),
            #[cfg(test)]
            fail_purge_wallet: std::sync::atomic::AtomicBool::new(false),
        };
        store.rehydrate_pending_spends()?;
        Ok(store)
    }

    /// Add `shielded_pending_spends.identity_index` to a database created
    /// before that column existed (#4313 review finding 5d4d6efa).
    ///
    /// This store versions its schema by `CREATE TABLE IF NOT EXISTS` rather
    /// than by `user_version`, so the matching idempotent form for a new column
    /// is "read `PRAGMA table_info` and add it if absent". The column is
    /// NULLABLE with no default: an existing claim record genuinely does not
    /// know which slot its attempt targeted, and `NULL` says exactly that —
    /// far better than back-filling a guess a resume would then enforce.
    ///
    /// # Racing opens
    ///
    /// Probe-then-ALTER is only idempotent if the two are ONE step. Two
    /// processes (or two `FileBackedShieldedStore` instances) opening the same
    /// path concurrently would otherwise both read "absent" and both ALTER,
    /// and the loser's `open_path` would fail outright with
    /// `duplicate column name` (#4313 review finding file_store.rs:206). Two
    /// independent guards close that:
    ///
    /// 1. `BEGIN IMMEDIATE` takes the write lock BEFORE the probe, so SQLite's
    ///    one-writer rule totally orders the probe+ALTER pairs against each
    ///    other — the second one to run sees the column and does nothing.
    /// 2. A `duplicate column name` failure is tolerated as benign anyway.
    ///    The post-condition this function owes its caller is "the column
    ///    exists", and that error says it does. Belt and braces, because the
    ///    cost of being wrong is a store that will not open at all.
    fn add_pending_spends_identity_index(
        conn: &mut Connection,
    ) -> Result<(), FileShieldedStoreError> {
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| FileShieldedStoreError(format!("begin pending_spends migration: {e}")))?;
        let present = {
            let mut stmt = tx
                .prepare("SELECT 1 FROM pragma_table_info('shielded_pending_spends') WHERE name = 'identity_index'")
                .map_err(|e| {
                    FileShieldedStoreError(format!("prepare pending_spends column probe: {e}"))
                })?;
            stmt.exists([])
                .map_err(|e| FileShieldedStoreError(format!("probe pending_spends columns: {e}")))?
        };
        if !present {
            match tx.execute(
                "ALTER TABLE shielded_pending_spends ADD COLUMN identity_index INTEGER",
                [],
            ) {
                Ok(_) => {}
                Err(e) if Self::is_duplicate_column(&e) => {
                    // Guard 2 above: another opener won the race and the
                    // column is already there, which is exactly the state
                    // this function exists to reach.
                    tracing::debug!(
                        "shielded_pending_spends.identity_index already added by a concurrent \
                         opener; treating as migrated"
                    );
                }
                Err(e) => {
                    return Err(FileShieldedStoreError(format!(
                        "add pending_spends.identity_index: {e}"
                    )))
                }
            }
        }
        tx.commit()
            .map_err(|e| FileShieldedStoreError(format!("commit pending_spends migration: {e}")))?;
        Ok(())
    }

    /// Whether `e` is SQLite's `duplicate column name` rejection of an
    /// `ALTER TABLE ... ADD COLUMN` — i.e. "the column you asked for is
    /// already there".
    ///
    /// Matched on the message rather than on a code: SQLite reports it as a
    /// bare `SQLITE_ERROR` with no distinguishing extended code, so the text
    /// is the only discriminator available. Deliberately narrow — every other
    /// `SQLITE_ERROR` still fails the open.
    fn is_duplicate_column(e: &rusqlite::Error) -> bool {
        matches!(
            e,
            rusqlite::Error::SqliteFailure(_, Some(msg))
                if msg.to_ascii_lowercase().contains("duplicate column name")
        )
    }

    /// Lock the durable pending/lifecycle connection, mapping mutex
    /// poisoning into the store's typed error instead of panicking.
    ///
    /// Every caller here already returns [`FileShieldedStoreError`], and the
    /// commitment-tree mutex has always mapped `PoisonError` this way. The
    /// pending-connection sites used to `.expect(...)`, which meant that a
    /// panic caught anywhere while this lock was held turned every later
    /// claim, purge, migration, or admission call into a second panic rather
    /// than a `ShieldedStore::Error` — and through `block_on_worker` that
    /// secondary panic is re-raised into the host and can abort the process
    /// (#4313 review finding file_store.rs:1175).
    ///
    /// A poisoned lock is reported, not recovered: the interrupted writer may
    /// have left a claim-recovery row half-written, so failing closed is the
    /// safe direction — callers surface a retryable store error and the row
    /// stays on disk for the next open to rehydrate.
    fn lock_pending_conn(&self) -> Result<MutexGuard<'_, Connection>, FileShieldedStoreError> {
        self.pending_conn
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("pending_conn mutex poisoned: {e}")))
    }

    /// Make [`ShieldedStore::purge_wallet`] fail, and ONLY it, until
    /// [`allow_purge_wallet_for_tests`](Self::allow_purge_wallet_for_tests)
    /// lifts it.
    ///
    /// The purge is the one fallible step that runs AFTER a removal's commit
    /// point, so its failure branch is unreachable while the store is healthy
    /// and cannot be produced by the pre-commit admission drain either. A
    /// blunter seam (`PRAGMA query_only`) fails the admission write too, which
    /// aborts the removal BEFORE the commit point and exercises the opposite
    /// contract — hence the targeted flag.
    #[cfg(test)]
    pub(crate) fn fail_purge_wallet_for_tests(&self) {
        self.fail_purge_wallet
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Undo [`fail_purge_wallet_for_tests`](Self::fail_purge_wallet_for_tests).
    #[cfg(test)]
    pub(crate) fn allow_purge_wallet_for_tests(&self) {
        self.fail_purge_wallet
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    /// Whether a durable pending row exists for `id` / `activity_id`, read
    /// straight from SQLite rather than from the in-memory mirror.
    #[cfg(test)]
    pub(crate) fn has_pending_row_for_tests(
        &self,
        wallet_id: WalletId,
    ) -> Result<bool, FileShieldedStoreError> {
        let conn = self.lock_pending_conn()?;
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM shielded_pending_spends WHERE wallet_id = ?1",
                rusqlite::params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .map_err(|e| FileShieldedStoreError(format!("count pending rows: {e}")))?;
        Ok(count > 0)
    }

    /// Reload every persisted [`PendingRedrive`] into the in-memory
    /// per-subwallet state, re-arming both the redrive record and the
    /// note reservations its nullifiers carry — an unconfirmed
    /// broadcast therefore keeps its notes reserved (and its re-drive
    /// alive) across restarts. Corrupt rows are dropped with a warning
    /// rather than failing the open.
    fn rehydrate_pending_spends(&mut self) -> Result<(), FileShieldedStoreError> {
        // The rows are read out under the lock and the guard is dropped before
        // `self.subwallets` is touched. `lock_pending_conn` borrows all of
        // `self` (it is a method, where the old inline `self.pending_conn.lock()`
        // borrowed just the field), so holding the guard across the hydration
        // loop below would conflict with the `&mut self.subwallets` it needs.
        let raw_rows: Vec<PendingSpendRow> = {
            let conn = self.lock_pending_conn()?;
            let mut stmt = conn
                .prepare(
                    "SELECT wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, \
                     attempts, identity_index FROM shielded_pending_spends",
                )
                .map_err(|e| FileShieldedStoreError(format!("prepare rehydrate: {e}")))?;
            let rows = stmt
                .query_map([], |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                        row.get::<_, Vec<u8>>(4)?,
                        row.get::<_, Vec<u8>>(5)?,
                        row.get::<_, u32>(6)?,
                        row.get::<_, Option<u32>>(7)?,
                    ))
                })
                .map_err(|e| FileShieldedStoreError(format!("query rehydrate: {e}")))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| FileShieldedStoreError(format!("read rehydrate row: {e}")))?
        };
        for row in raw_rows {
            let (
                wallet_id,
                account_index,
                activity_id,
                anchor,
                nullifiers,
                st_bytes,
                attempts,
                identity_index,
            ) = row;
            let (Ok(wallet_id), Ok(activity_id), Ok(anchor)) = (
                <[u8; 32]>::try_from(wallet_id.as_slice()),
                <[u8; 32]>::try_from(activity_id.as_slice()),
                <[u8; 32]>::try_from(anchor.as_slice()),
            ) else {
                tracing::warn!("dropping corrupt shielded_pending_spends row (bad key widths)");
                continue;
            };
            if nullifiers.is_empty() || nullifiers.len() % 32 != 0 {
                tracing::warn!("dropping corrupt shielded_pending_spends row (bad nullifiers)");
                continue;
            }
            let nullifiers: Vec<[u8; 32]> = nullifiers
                .chunks_exact(32)
                .map(|c| <[u8; 32]>::try_from(c).expect("chunks_exact(32)"))
                .collect();
            let id = SubwalletId::new(wallet_id, account_index);
            Self::hydrate_pending_row(
                self.subwallets.entry(id).or_default(),
                PendingRedrive {
                    activity_id,
                    anchor,
                    nullifiers,
                    st_bytes,
                    attempts,
                    identity_index,
                },
            );
        }
        Ok(())
    }

    /// Read ONE `shielded_pending_spends` row — the record armed under
    /// `activity_id` in subwallet `id` — straight from SQLite.
    ///
    /// Takes a `&Connection` (a `&Transaction` derefs to one) precisely so the
    /// caller chooses the transaction it runs in:
    /// [`reserve_one_time_claim_key`] calls it inside the reservation's
    /// `BEGIN IMMEDIATE`, which is what makes "who owns this invitation" and
    /// "what record already exists for it" a single atomic answer
    /// (#4313 review finding r3767229122).
    ///
    /// A corrupt row reads as `None` with a warning, matching
    /// [`rehydrate_pending_spends`]: a row that cannot be decoded cannot be
    /// resumed either, and failing the whole claim on it would be worse than
    /// rebuilding.
    ///
    /// [`reserve_one_time_claim_key`]: ShieldedStore::reserve_one_time_claim_key
    /// [`rehydrate_pending_spends`]: Self::rehydrate_pending_spends
    fn read_pending_row(
        conn: &Connection,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<Option<PendingRedrive>, FileShieldedStoreError> {
        let row = conn
            .query_row(
                "SELECT anchor, nullifiers, st_bytes, attempts, identity_index \
                 FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    activity_id.as_slice()
                ],
                |row| {
                    Ok((
                        row.get::<_, Vec<u8>>(0)?,
                        row.get::<_, Vec<u8>>(1)?,
                        row.get::<_, Vec<u8>>(2)?,
                        row.get::<_, u32>(3)?,
                        row.get::<_, Option<u32>>(4)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| FileShieldedStoreError(format!("read pending claim record: {e}")))?;
        let Some((anchor, nullifiers, st_bytes, attempts, identity_index)) = row else {
            return Ok(None);
        };
        let Ok(anchor) = <[u8; 32]>::try_from(anchor.as_slice()) else {
            tracing::warn!("ignoring corrupt shielded_pending_spends row (bad anchor width)");
            return Ok(None);
        };
        if nullifiers.is_empty() || nullifiers.len() % 32 != 0 {
            tracing::warn!("ignoring corrupt shielded_pending_spends row (bad nullifiers)");
            return Ok(None);
        }
        Ok(Some(PendingRedrive {
            activity_id: *activity_id,
            anchor,
            nullifiers: nullifiers
                .chunks_exact(32)
                .map(|c| <[u8; 32]>::try_from(c).expect("chunks_exact(32)"))
                .collect(),
            st_bytes,
            attempts,
            identity_index,
        }))
    }

    /// Fold a durable `shielded_pending_spends` row into the in-memory mirror:
    /// re-arm the redrive record AND the note reservations its nullifiers
    /// carry, so an unconfirmed broadcast keeps its notes excluded from
    /// selection for as long as the record lives.
    ///
    /// Shared by [`rehydrate_pending_spends`] (store open) and by
    /// [`reserve_one_time_claim_key`] (a row a PEER store armed after our
    /// open), so both reach the identical in-memory shape.
    ///
    /// [`rehydrate_pending_spends`]: Self::rehydrate_pending_spends
    /// [`reserve_one_time_claim_key`]: ShieldedStore::reserve_one_time_claim_key
    fn hydrate_pending_row(sw: &mut SubwalletState, record: PendingRedrive) {
        for n in &record.nullifiers {
            sw.mark_pending(n);
            sw.set_pending_spend(n, record.anchor, record.activity_id);
        }
        sw.arm_redrive(record);
    }

    /// Open a `rusqlite::Connection` on `path` with the same WAL /
    /// `synchronous=NORMAL` / `temp_store=MEMORY` PRAGMAs the cold-sync
    /// append path depends on (see [`open_path`] for the rationale).
    ///
    /// Shared by [`open_path`] and [`reset_commitment_tree`] so any
    /// connection the store hands to `ClientPersistentCommitmentTree`
    /// — original or post-reset — is configured identically.
    ///
    /// [`open_path`]: Self::open_path
    /// [`reset_commitment_tree`]: ShieldedStore::reset_commitment_tree
    fn open_tuned_connection(path: &Path) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        Self::open_connection_with_sync(path, "NORMAL")
    }

    /// Open the RECOVERY connection — the one owning `shielded_pending_spends`
    /// and the admission tables — with `synchronous=FULL` rather than the
    /// commitment tree connection's `NORMAL`.
    ///
    /// # Why this connection alone pays for FULL
    ///
    /// Under WAL, `synchronous=NORMAL` does not fsync at commit: the commit
    /// returns as soon as the frames reach the OS, so a host crash or power
    /// loss can discard a transaction that already reported success. That is
    /// the right trade for the commitment tree, where no row is user money —
    /// every commitment is chain-side authenticated and rebuildable by
    /// re-running sync from `last_synced_note_index` (see [`open_path`]).
    ///
    /// It is the WRONG trade for the row this connection writes.
    /// [`arm_redrive_under_claim`] persists a one-time-claim record whose
    /// `st_bytes` carry the RANDOMIZED padded identity id of a transition
    /// that is broadcast immediately afterwards: the padding action's dummy
    /// nullifier is generated fresh at build time and participates in the
    /// consensus id derivation, so that id exists nowhere else and is NOT
    /// re-derivable from the invitation. Losing the row after the broadcast
    /// therefore strands an identity that exists on chain, permanently and
    /// unreconstructably (#4313 review finding file_store.rs:107). FULL
    /// closes the window by fsync'ing before the commit returns.
    ///
    /// The cost lands where it is affordable: a handful of writes per claim
    /// (arm, lease renew, release) rather than the tree's millions of
    /// `append_commitment` calls. `synchronous` is per-CONNECTION, so the
    /// tree connection keeps NORMAL; `journal_mode=WAL` is per-database and
    /// shared by both.
    ///
    /// [`open_path`]: Self::open_path
    /// [`arm_redrive_under_claim`]: ShieldedStore::arm_redrive_under_claim
    fn open_durable_connection(
        path: &Path,
    ) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        Self::open_connection_with_sync(path, "FULL")
    }

    /// Shared body of [`open_tuned_connection`] and
    /// [`open_durable_connection`] — identical WAL / `temp_store` / busy-timeout
    /// setup, with the caller choosing the `synchronous` level its data needs.
    ///
    /// [`open_tuned_connection`]: Self::open_tuned_connection
    /// [`open_durable_connection`]: Self::open_durable_connection
    fn open_connection_with_sync(
        path: &Path,
        synchronous: &str,
    ) -> Result<rusqlite::Connection, FileShieldedStoreError> {
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| FileShieldedStoreError(format!("open sqlite: {e}")))?;
        // Pragmas must be applied before the schema is touched. They survive
        // for the lifetime of the connection; WAL also persists for any
        // subsequent reopen on the same file until explicitly changed.
        for (k, v) in [
            ("journal_mode", "WAL"),
            ("synchronous", synchronous),
            ("temp_store", "MEMORY"),
        ] {
            conn.pragma_update(None, k, v)
                .map_err(|e| FileShieldedStoreError(format!("PRAGMA {k}={v}: {e}")))?;
        }
        // Two writer connections share this file (the commitment tree's and
        // `pending_conn`). WAL allows one writer at a time; without a busy
        // timeout a write colliding with the other connection's write txn
        // fails immediately with SQLITE_BUSY instead of briefly waiting.
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(|e| FileShieldedStoreError(format!("busy_timeout: {e}")))?;
        Ok(conn)
    }

    /// Unix millis as SQLite's native signed 64-bit integer.
    ///
    /// Saturating rather than wrapping: a caller that adds an absurd lease to
    /// `now` must produce a far-future deadline, never a negative one that
    /// would read as already expired and silently drop the fence.
    fn as_sqlite_millis(millis: u64) -> i64 {
        i64::try_from(millis).unwrap_or(i64::MAX)
    }

    /// Drop every admission whose deadline has passed.
    ///
    /// Called at the top of both admission-taking transactions, so a holder
    /// that died — process kill, cancelled coroutine — cannot block the other
    /// side forever. This is a LIVENESS backstop only: it never removes a live
    /// admission, so it cannot let a purge delete a record out from under a
    /// claim that is still running.
    fn reap_expired_admissions(
        tx: &rusqlite::Transaction<'_>,
        now_ms: u64,
    ) -> Result<(), FileShieldedStoreError> {
        tx.execute(
            "DELETE FROM shielded_lifecycle_admission WHERE expires_at <= ?1",
            rusqlite::params![Self::as_sqlite_millis(now_ms)],
        )
        .map_err(|e| FileShieldedStoreError(format!("reap expired admissions: {e}")))?;
        Ok(())
    }

    /// Delete the single persisted redrive row for `id` keyed by
    /// `activity_id`. Used to mirror the exact in-memory drops
    /// [`SubwalletState::mark_spent`] reports, avoiding the
    /// scan-every-row cost of [`Self::delete_redrive_rows_containing`]
    /// on the common path where the resolved note had no armed redrive.
    fn delete_redrive_row(
        &self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<(), FileShieldedStoreError> {
        let conn = self.lock_pending_conn()?;
        conn.execute(
            "DELETE FROM shielded_pending_spends \
             WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
            rusqlite::params![
                id.wallet_id.as_slice(),
                id.account_index,
                activity_id.as_slice(),
            ],
        )
        .map_err(|e| FileShieldedStoreError(format!("delete redrive row by activity: {e}")))?;
        Ok(())
    }

    /// Mirror to SQLite the redrive deletions [`SubwalletState`] performs
    /// in memory when a nullifier resolves via `clear_pending`: delete
    /// every persisted row for `id` whose nullifier blob contains
    /// `nullifier`.
    fn delete_redrive_rows_containing(
        &self,
        id: SubwalletId,
        nullifier: &[u8; 32],
    ) -> Result<(), FileShieldedStoreError> {
        let conn = self.lock_pending_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT activity_id, nullifiers FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2",
            )
            .map_err(|e| FileShieldedStoreError(format!("prepare redrive lookup: {e}")))?;
        let rows: Vec<(Vec<u8>, Vec<u8>)> = stmt
            .query_map(
                rusqlite::params![id.wallet_id.as_slice(), id.account_index],
                |row| Ok((row.get::<_, Vec<u8>>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
            .map_err(|e| FileShieldedStoreError(format!("query redrive lookup: {e}")))?
            .collect::<Result<_, _>>()
            .map_err(|e| FileShieldedStoreError(format!("read redrive lookup: {e}")))?;
        drop(stmt);
        for (activity_id, nullifiers) in rows {
            if nullifiers
                .chunks_exact(32)
                .any(|c| c == nullifier.as_slice())
            {
                conn.execute(
                    "DELETE FROM shielded_pending_spends \
                     WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
                    rusqlite::params![id.wallet_id.as_slice(), id.account_index, activity_id],
                )
                .map_err(|e| FileShieldedStoreError(format!("delete redrive row: {e}")))?;
            }
        }
        Ok(())
    }
}

impl ShieldedStore for FileBackedShieldedStore {
    type Error = FileShieldedStoreError;

    fn save_note(&mut self, id: SubwalletId, note: &ShieldedNote) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_note(note);
        Ok(())
    }

    fn get_unspent_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::unspent_notes)
            .unwrap_or_default())
    }

    fn get_all_notes(&self, id: SubwalletId) -> Result<Vec<ShieldedNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::all_notes)
            .unwrap_or_default())
    }

    fn mark_spent(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(false);
        };
        let outcome = sw.mark_spent(nullifier);
        // Mirror the durable deletion whenever the in-memory drop
        // happened — keyed on the returned activity ids, NOT on
        // `newly_spent`. A note restored already-spent still resolves a
        // rehydrated redrive here (`newly_spent == false`), and leaving
        // the SQLite row would resurrect the reservation on the next
        // open. Targeting the exact activity ids means the common case
        // (no armed redrive for this note) issues zero SQLite work. The
        // in-memory transition already happened, so a SQLite failure
        // only warns — a surviving row rehydrates and self-heals via the
        // reconcile / prune passes.
        for activity_id in &outcome.dropped_redrives {
            if let Err(e) = self.delete_redrive_row(id, activity_id) {
                tracing::warn!(
                    error = %e,
                    "redrive row deletion failed after mark_spent; a stale row may \
                     rehydrate on the next open (self-heals via reconcile/prune)"
                );
            }
        }
        Ok(outcome.newly_spent)
    }

    fn mark_pending(&mut self, id: SubwalletId, nullifier: &[u8; 32]) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .mark_pending(nullifier))
    }

    fn clear_pending(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
    ) -> Result<bool, Self::Error> {
        let Some(sw) = self.subwallets.get_mut(&id) else {
            return Ok(false);
        };
        let removed = sw.clear_pending(nullifier);
        if removed {
            // Same log-don't-abort rationale as `mark_spent` above.
            if let Err(e) = self.delete_redrive_rows_containing(id, nullifier) {
                tracing::warn!(
                    error = %e,
                    "redrive row deletion failed after clear_pending; a stale row may \
                     rehydrate on the next open (self-heals via reconcile/prune)"
                );
            }
        }
        Ok(removed)
    }

    fn set_pending_spend(
        &mut self,
        id: SubwalletId,
        nullifier: &[u8; 32],
        anchor: [u8; 32],
        activity_id: [u8; 32],
    ) -> Result<(), Self::Error> {
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.set_pending_spend(nullifier, anchor, activity_id);
        }
        Ok(())
    }

    fn stale_pending_spends(&self, id: SubwalletId) -> Result<Vec<StalePendingSpend>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::stale_pending_spends)
            .unwrap_or_default())
    }

    fn arm_redrive(&mut self, id: SubwalletId, redrive: PendingRedrive) -> Result<(), Self::Error> {
        {
            let conn = self.lock_pending_conn()?;
            let nullifier_blob: Vec<u8> = redrive.nullifiers.iter().flatten().copied().collect();
            conn.execute(
                "INSERT OR REPLACE INTO shielded_pending_spends \
                 (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts, \
                  identity_index) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    redrive.activity_id.as_slice(),
                    redrive.anchor.as_slice(),
                    nullifier_blob,
                    redrive.st_bytes,
                    redrive.attempts,
                    redrive.identity_index,
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("persist redrive: {e}")))?;
        }
        self.subwallets.entry(id).or_default().arm_redrive(redrive);
        Ok(())
    }

    fn pending_redrives(&self, id: SubwalletId) -> Result<Vec<PendingRedrive>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::pending_redrives)
            .unwrap_or_default())
    }

    fn bump_redrive_attempts(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<u32, Self::Error> {
        // Persist FIRST, mutate memory only on success: the reverse order
        // would leave the in-memory counter ahead of the durable row on a
        // SQLite failure, and a restart would rewind the attempt budget.
        let Some(next) = self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.redrive_attempts(activity_id))
            .map(|attempts| attempts + 1)
        else {
            return Ok(0);
        };
        {
            let conn = self.lock_pending_conn()?;
            conn.execute(
                "UPDATE shielded_pending_spends SET attempts = ?4 \
                 WHERE wallet_id = ?1 AND account_index = ?2 AND activity_id = ?3",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    activity_id.as_slice(),
                    next,
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("bump redrive attempts: {e}")))?;
        }
        let attempts = self
            .subwallets
            .get_mut(&id)
            .map(|sw| sw.bump_redrive_attempts(activity_id))
            .unwrap_or(0);
        Ok(attempts)
    }

    fn clear_redrive(
        &mut self,
        id: SubwalletId,
        activity_id: &[u8; 32],
    ) -> Result<(), Self::Error> {
        if let Some(sw) = self.subwallets.get_mut(&id) {
            sw.clear_redrive(activity_id);
        }
        self.delete_redrive_row(id, activity_id)
    }

    fn record_outgoing_note(
        &mut self,
        id: SubwalletId,
        note: &ShieldedOutgoingNote,
    ) -> Result<bool, Self::Error> {
        Ok(self
            .subwallets
            .entry(id)
            .or_default()
            .record_outgoing_note(note))
    }

    fn get_outgoing_notes(
        &self,
        id: SubwalletId,
    ) -> Result<Vec<ShieldedOutgoingNote>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::outgoing_notes)
            .unwrap_or_default())
    }

    fn save_activity(
        &mut self,
        id: SubwalletId,
        entry: &super::activity::ShieldedActivityEntry,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().save_activity(entry);
        Ok(())
    }

    fn get_activity(
        &self,
        id: SubwalletId,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.activity_page(offset, limit))
            .unwrap_or_default())
    }

    fn get_activity_by_entry_id(
        &self,
        id: SubwalletId,
        entry_id: &[u8; 32],
    ) -> Result<Option<super::activity::ShieldedActivityEntry>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .and_then(|sw| sw.activity_by_id(entry_id)))
    }

    fn get_activity_ids(
        &self,
        id: SubwalletId,
    ) -> Result<std::collections::BTreeSet<[u8; 32]>, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(SubwalletState::activity_ids)
            .unwrap_or_default())
    }

    fn append_commitment(&mut self, cmx: &[u8; 32], marked: bool) -> Result<(), Self::Error> {
        let retention: Retention<u32> = if marked {
            Retention::Marked
        } else {
            Retention::Ephemeral
        };
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.append(*cmx, retention)
            .map_err(|e| FileShieldedStoreError(format!("append commitment: {e}")))
    }

    fn checkpoint_tree(&mut self, checkpoint_id: u32) -> Result<(), Self::Error> {
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.checkpoint(checkpoint_id)
            .map(|_| ())
            .map_err(|e| FileShieldedStoreError(format!("checkpoint tree: {e}")))
    }

    fn tree_anchor(&self) -> Result<[u8; 32], Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        tree.anchor()
            .map(|a| a.to_bytes())
            .map_err(|e| FileShieldedStoreError(format!("read tree anchor: {e}")))
    }

    fn witness_at_depth(
        &self,
        position: u64,
        depth: usize,
    ) -> Result<Option<grovedb_commitment_tree::MerklePath>, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        // `checkpoint_depth = 0` is the current tree state; deeper values
        // reach older checkpoints so a spend can be built against a root
        // Platform actually recorded (it records one anchor per block, while
        // an index-chunk sync routinely leaves the tree mid-block). The proof
        // uses whichever anchor this witness produces via `MerklePath::root`,
        // so the anchor and the authentication path always agree.
        tree.witness(Position::from(position), depth)
            .map_err(|e| FileShieldedStoreError(format!("witness({position}, depth {depth}): {e}")))
    }

    fn tree_size(&self) -> Result<u64, Self::Error> {
        let tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;
        let size = tree
            .max_leaf_position()
            .map_err(|e| FileShieldedStoreError(format!("read tree size: {e}")))?
            .map(|p| u64::from(p) + 1)
            .unwrap_or(0);
        Ok(size)
    }

    fn last_synced_note_index(&self, id: SubwalletId) -> Result<u64, Self::Error> {
        Ok(self
            .subwallets
            .get(&id)
            .map(|sw| sw.last_synced_index)
            .unwrap_or(0))
    }

    fn set_last_synced_note_index(
        &mut self,
        id: SubwalletId,
        index: u64,
    ) -> Result<(), Self::Error> {
        self.subwallets.entry(id).or_default().last_synced_index = index;
        Ok(())
    }

    fn purge_wallet(&mut self, wallet_id: WalletId) -> Result<(), Self::Error> {
        #[cfg(test)]
        if self
            .fail_purge_wallet
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Err(FileShieldedStoreError(
                "purge pending spends for wallet: injected test failure".to_string(),
            ));
        }
        // The redrive table IS durable (unlike the rest of subwallet
        // state), so purging the in-memory map alone would leave this
        // wallet's rows to rehydrate stale reservations / rebroadcast
        // state on the next open. Delete them first, scoped by
        // wallet_id — SQL before memory, so an Err return means neither
        // store was touched (fail-atomic) rather than a memory purge
        // the caller can't distinguish from a no-op.
        {
            let conn = self.lock_pending_conn()?;
            conn.execute(
                "DELETE FROM shielded_pending_spends WHERE wallet_id = ?1",
                rusqlite::params![wallet_id.as_slice()],
            )
            .map_err(|e| FileShieldedStoreError(format!("purge pending spends for wallet: {e}")))?;
        }
        // Per-subwallet note / watermark / checkpoint state is
        // in-memory only (`subwallets`); the commitment tree in
        // SQLite is chain-wide and intentionally left intact.
        self.subwallets.retain(|id, _| id.wallet_id != wallet_id);
        Ok(())
    }

    fn purge_subwallet(&mut self, id: SubwalletId) -> Result<(), Self::Error> {
        // Durable redrive rows are scoped by (wallet_id, account_index);
        // delete this subwallet's before the in-memory drop, same
        // SQL-before-memory fail-atomic ordering as `purge_wallet`.
        {
            let conn = self.lock_pending_conn()?;
            conn.execute(
                "DELETE FROM shielded_pending_spends \
                 WHERE wallet_id = ?1 AND account_index = ?2",
                rusqlite::params![id.wallet_id.as_slice(), id.account_index],
            )
            .map_err(|e| {
                FileShieldedStoreError(format!("purge pending spends for subwallet: {e}"))
            })?;
        }
        self.subwallets.remove(&id);
        Ok(())
    }

    fn purge_all_subwallets(&mut self) -> Result<(), Self::Error> {
        // Durable redrive rows for every wallet go with the in-memory
        // purge; SQL first for the same fail-atomic reason as
        // `purge_wallet`.
        {
            let conn = self.lock_pending_conn()?;
            conn.execute("DELETE FROM shielded_pending_spends", [])
                .map_err(|e| FileShieldedStoreError(format!("purge all pending spends: {e}")))?;
        }
        self.subwallets.clear();
        Ok(())
    }

    fn reset_commitment_tree(&mut self) -> Result<(), Self::Error> {
        // The `ClientPersistentCommitmentTree` wrapper owns its
        // `Connection` and exposes no public truncate (only the inner
        // `SqliteShardStore` has `truncate_shards`). A full reset
        // therefore (1) wipes the four `commitment_tree_*` tables on a
        // fresh connection, then (2) rebuilds the wrapper over the now
        // empty DB so the in-memory shardtree frontier/cap reflect the
        // empty state. Reopening — rather than mutating the live tree —
        // is what guarantees `tree_size()` reads back 0: the wrapper
        // caches frontier nodes that a bare `DELETE` wouldn't clear.
        let mut tree = self
            .tree
            .lock()
            .map_err(|e| FileShieldedStoreError(format!("tree mutex poisoned: {e}")))?;

        {
            let conn = Self::open_tuned_connection(&self.path)?;
            // `commitment_tree_cap` is included alongside the three
            // shard/checkpoint tables: it caches upper-level tree nodes,
            // so leaving it populated while the shards are empty would
            // reopen into an inconsistent (non-empty) tree state.
            conn.execute_batch(
                "DELETE FROM commitment_tree_checkpoint_marks_removed;
                 DELETE FROM commitment_tree_checkpoints;
                 DELETE FROM commitment_tree_shards;
                 DELETE FROM commitment_tree_cap;",
            )
            .map_err(|e| FileShieldedStoreError(format!("reset commitment tree tables: {e}")))?;

            // Durably flush the DELETEs into the main database file with a
            // TRUNCATE checkpoint before this connection is dropped.
            //
            // Without this, Clear is a no-op across a hard kill. The store
            // runs `synchronous=NORMAL` in WAL mode (see `open_tuned_connection`),
            // so a committed transaction lands in the `-wal` file but is NOT
            // fsync'd until a checkpoint. Two other connections (`tree` and
            // `pending_conn`) stay open on the same file, so SQLite's
            // last-connection-close auto-checkpoint never fires when this
            // transient connection drops — the emptied tables live only in the
            // WAL. On Android the "Clear" button is routinely followed by a
            // force-stop (non-graceful SIGKILL, no checkpoint), so the WAL
            // frames are discarded and the next launch reopens the OLD full
            // tree (the 771/771 "Clear did nothing" symptom). A TRUNCATE
            // checkpoint rewrites the main db and resets the WAL, making the
            // emptied state durable regardless of how the process later dies.
            // The prior graceful `drop(store)` in the unit test masked this —
            // a clean close checkpoints, a SIGKILL does not.
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
                .map_err(|e| FileShieldedStoreError(format!("checkpoint after tree reset: {e}")))?;
        }

        let conn = Self::open_tuned_connection(&self.path)?;
        *tree = ClientPersistentCommitmentTree::open(conn, self.max_checkpoints)
            .map_err(|e| FileShieldedStoreError(format!("reopen commitment tree: {e}")))?;
        Ok(())
    }

    // ── Lifecycle admission ────────────────────────────────────────────
    //
    // Every method below runs its whole check-and-write inside ONE
    // `BEGIN IMMEDIATE` transaction. That is the entire correctness argument:
    // SQLite admits a single write transaction at a time across every
    // connection AND every process on the file, so `begin_claim_admission` and
    // `begin_destructive_admission` are totally ordered even between two
    // `FileBackedShieldedStore` instances that share nothing else. See
    // `store::LifecycleAdmission` for both orders and why each is safe.
    //
    // `busy_timeout` (5 s, set in `open_tuned_connection`) absorbs contention;
    // no transaction here spans an await, a scan, a proof or a broadcast.

    fn begin_claim_admission(
        &mut self,
        wallet_id: WalletId,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        let mut conn = self.lock_pending_conn()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| FileShieldedStoreError(format!("begin claim admission: {e}")))?;
        Self::reap_expired_admissions(&tx, now_ms)?;
        // A store-wide barrier (`wallet_id IS NULL`, from `clear`) covers every
        // wallet; a scoped one covers only its own.
        let blocked: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM shielded_lifecycle_admission \
                 WHERE destructive = 1 AND (wallet_id IS NULL OR wallet_id = ?1)",
                rusqlite::params![wallet_id.as_slice()],
                |row| row.get(0),
            )
            .map_err(|e| FileShieldedStoreError(format!("read destructive barriers: {e}")))?;
        if blocked > 0 {
            // Return WITHOUT committing: dropping the `Transaction` rolls it
            // back, so a refused claim leaves no lease row and no half-open
            // admission behind (the reap above is rolled back with it, which
            // is harmless — the next admission call reaps again).
            return Ok(false);
        }
        tx.execute(
            "INSERT OR REPLACE INTO shielded_lifecycle_admission \
             (token, destructive, wallet_id, expires_at) VALUES (?1, 0, ?2, ?3)",
            rusqlite::params![
                token.0.as_slice(),
                wallet_id.as_slice(),
                Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
            ],
        )
        .map_err(|e| FileShieldedStoreError(format!("insert claim lease: {e}")))?;
        tx.commit()
            .map_err(|e| FileShieldedStoreError(format!("commit claim admission: {e}")))?;
        Ok(true)
    }

    fn renew_claim_admission(
        &mut self,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        // IMMEDIATE, like every other lease write: SQLite's one-writer rule is
        // what totally orders this against a purge taking its barrier, so the
        // renewal either lands before the barrier or loses to it — never
        // half-applies. UPDATE ... WHERE expires_at > now deliberately refuses
        // to resurrect a lapsed lease; see the trait docs.
        let mut conn = self.lock_pending_conn()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| FileShieldedStoreError(format!("begin claim lease renewal: {e}")))?;
        let updated = tx
            .execute(
                "UPDATE shielded_lifecycle_admission SET expires_at = ?1 \
                 WHERE token = ?2 AND destructive = 0 AND expires_at > ?3",
                rusqlite::params![
                    Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                    token.0.as_slice(),
                    Self::as_sqlite_millis(now_ms)
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("renew claim lease: {e}")))?;
        if updated > 0 {
            // Keep the claim-key reservation in lockstep with the lease that
            // owns it, in the SAME transaction — a long claim must not lose its
            // invitation to expiry while its lease is being kept alive.
            tx.execute(
                "UPDATE shielded_one_time_claim_reservation SET expires_at = ?1 WHERE token = ?2",
                rusqlite::params![
                    Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                    token.0.as_slice(),
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("renew claim-key reservation: {e}")))?;
        }
        tx.commit()
            .map_err(|e| FileShieldedStoreError(format!("commit claim lease renewal: {e}")))?;
        Ok(updated > 0)
    }

    fn reserve_one_time_claim_key(
        &mut self,
        claim_records_id: SubwalletId,
        claim_record_key: [u8; 32],
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<ClaimKeyReservationOutcome, Self::Error> {
        let wallet_id = claim_records_id.wallet_id;
        // BEGIN IMMEDIATE, like every other admission write. SQLite admits one
        // writer at a time across every connection AND every process on the
        // file, so the reap + insert-if-absent + read-back + pending-row read
        // below is one totally ordered step even between two
        // `FileBackedShieldedStore` instances that share nothing but the path.
        // That total order is what makes "exactly one caller sees `Acquired`"
        // true rather than probable — and what lets the pending row come back
        // with it (#4313 review finding r3767229122).
        let (reservation, pending) = {
            let mut conn = self.lock_pending_conn()?;
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| FileShieldedStoreError(format!("begin claim-key reservation: {e}")))?;
            // Reap first, so a claimant that died without releasing cannot hold
            // an invitation hostage past its lease.
            tx.execute(
                "DELETE FROM shielded_one_time_claim_reservation WHERE expires_at <= ?1",
                rusqlite::params![Self::as_sqlite_millis(now_ms)],
            )
            .map_err(|e| FileShieldedStoreError(format!("reap claim-key reservations: {e}")))?;
            // ON CONFLICT DO NOTHING, never OR REPLACE: losing this insert must
            // leave the winner's row byte-for-byte untouched. The rowcount
            // decides the outcome, and the read-back below reports the durable
            // truth either way.
            let inserted = tx
                .execute(
                    "INSERT INTO shielded_one_time_claim_reservation \
                     (wallet_id, claim_record_key, token, expires_at) VALUES (?1, ?2, ?3, ?4) \
                     ON CONFLICT (wallet_id, claim_record_key) DO NOTHING",
                    rusqlite::params![
                        wallet_id.as_slice(),
                        claim_record_key.as_slice(),
                        token.0.as_slice(),
                        Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                    ],
                )
                .map_err(|e| {
                    FileShieldedStoreError(format!("insert claim-key reservation: {e}"))
                })?;
            let reservation = if inserted > 0 {
                ClaimKeyReservation::Acquired
            } else {
                // Query the DURABLE row rather than assuming the conflict was
                // someone else's: our own token re-entering is idempotent (and
                // re-stamps), anyone else's is a genuine loss.
                let (holder, expires_at): (Vec<u8>, i64) = tx
                    .query_row(
                        "SELECT token, expires_at FROM shielded_one_time_claim_reservation \
                         WHERE wallet_id = ?1 AND claim_record_key = ?2",
                        rusqlite::params![wallet_id.as_slice(), claim_record_key.as_slice()],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(|e| {
                        FileShieldedStoreError(format!("read claim-key reservation: {e}"))
                    })?;
                let holder = <[u8; 16]>::try_from(holder.as_slice()).map_err(|_| {
                    FileShieldedStoreError(
                        "corrupt claim-key reservation row (bad token width)".to_string(),
                    )
                })?;
                if holder == token.0 {
                    tx.execute(
                        "UPDATE shielded_one_time_claim_reservation SET expires_at = ?1 \
                         WHERE wallet_id = ?2 AND claim_record_key = ?3",
                        rusqlite::params![
                            Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                            wallet_id.as_slice(),
                            claim_record_key.as_slice(),
                        ],
                    )
                    .map_err(|e| {
                        FileShieldedStoreError(format!("re-stamp claim-key reservation: {e}"))
                    })?;
                    ClaimKeyReservation::Acquired
                } else {
                    ClaimKeyReservation::Held {
                        holder: AdmissionToken(holder),
                        expires_at: expires_at.max(0) as u64,
                    }
                }
            };
            // The pending-claim row, read from SQLITE in this same transaction
            // (#4313 review finding r3767229122). It deliberately does NOT come
            // from `self.subwallets`: that mirror is hydrated once at store
            // open, so a row a PEER store armed after our open is invisible in
            // it. A claimant that trusted the mirror would see "no record",
            // build a second transition with a different padded identity id,
            // and `arm_redrive_under_claim` would replace the peer's only
            // recovery handle for an identity already on the wire.
            let pending = Self::read_pending_row(&tx, claim_records_id, &claim_record_key)?;
            tx.commit().map_err(|e| {
                FileShieldedStoreError(format!("commit claim-key reservation: {e}"))
            })?;
            (reservation, pending)
        };
        // Fold the durable row into this instance's mirror so every later
        // in-memory read (`pending_redrives`, `bump_redrive_attempts`,
        // `clear_redrive`) agrees with disk for the rest of this claim. Without
        // it the resume path would arm and clear against a map that never knew
        // the record existed.
        if let Some(record) = pending.clone() {
            Self::hydrate_pending_row(self.subwallets.entry(claim_records_id).or_default(), record);
        }
        Ok(ClaimKeyReservationOutcome {
            reservation,
            pending,
        })
    }

    fn arm_redrive_under_claim(
        &mut self,
        id: SubwalletId,
        redrive: PendingRedrive,
        token: AdmissionToken,
        now_ms: u64,
        lease_ms: u64,
    ) -> Result<bool, Self::Error> {
        {
            let mut conn = self.lock_pending_conn()?;
            let tx = conn
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|e| FileShieldedStoreError(format!("begin armed claim write: {e}")))?;
            // The claim-key gate, in the SAME transaction as the write it
            // guards: a live reservation for this exact record key under a
            // DIFFERENT token means another claimant owns this invitation, and
            // the `INSERT OR REPLACE` below would overwrite its byte-exact
            // recovery row. Refusing here is what makes that clobber
            // structurally impossible rather than merely unreachable
            // (#4313 review finding cr-9d0e1a44). Ordinary spend redrives take
            // no reservation, so the count is 0 and the gate is a no-op.
            let foreign_hold: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM shielded_one_time_claim_reservation \
                     WHERE wallet_id = ?1 AND claim_record_key = ?2 AND expires_at > ?3 \
                       AND token != ?4",
                    rusqlite::params![
                        id.wallet_id.as_slice(),
                        redrive.activity_id.as_slice(),
                        Self::as_sqlite_millis(now_ms),
                        token.0.as_slice(),
                    ],
                    |row| row.get(0),
                )
                .map_err(|e| FileShieldedStoreError(format!("read claim-key reservation: {e}")))?;
            if foreign_hold > 0 {
                return Ok(false);
            }
            let live: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM shielded_lifecycle_admission \
                     WHERE token = ?1 AND destructive = 0 AND expires_at > ?2",
                    rusqlite::params![token.0.as_slice(), Self::as_sqlite_millis(now_ms)],
                    |row| row.get(0),
                )
                .map_err(|e| FileShieldedStoreError(format!("read claim lease: {e}")))?;
            if live == 0 {
                // Lease gone (expired, or released). Write NOTHING and let the
                // caller fail closed — arming a record the store is no longer
                // holding open for us is how an in-flight claim loses its only
                // recovery handle.
                return Ok(false);
            }
            let nullifier_blob: Vec<u8> = redrive.nullifiers.iter().flatten().copied().collect();
            tx.execute(
                "INSERT OR REPLACE INTO shielded_pending_spends \
                 (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts, \
                  identity_index) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    redrive.activity_id.as_slice(),
                    redrive.anchor.as_slice(),
                    nullifier_blob,
                    redrive.st_bytes,
                    redrive.attempts,
                    redrive.identity_index,
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("persist claim record: {e}")))?;
            // Re-stamp in the SAME transaction, so the lease that admitted this
            // write is the one that covers the broadcast which follows it.
            tx.execute(
                "UPDATE shielded_lifecycle_admission SET expires_at = ?2 WHERE token = ?1",
                rusqlite::params![
                    token.0.as_slice(),
                    Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("restamp claim lease: {e}")))?;
            // The reservation rides the same re-stamp, for the same reason: the
            // window that protects the record it guards must run from here.
            tx.execute(
                "UPDATE shielded_one_time_claim_reservation SET expires_at = ?2 WHERE token = ?1",
                rusqlite::params![
                    token.0.as_slice(),
                    Self::as_sqlite_millis(now_ms.saturating_add(lease_ms)),
                ],
            )
            .map_err(|e| FileShieldedStoreError(format!("restamp claim-key reservation: {e}")))?;
            tx.commit()
                .map_err(|e| FileShieldedStoreError(format!("commit armed claim write: {e}")))?;
        }
        self.subwallets.entry(id).or_default().arm_redrive(redrive);
        Ok(true)
    }

    fn end_claim_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error> {
        // Lease and claim-key reservation drop together, in one transaction:
        // releasing the lease while leaving the key reserved would block the
        // next claimant of this invitation for a full lease period for no
        // reason, and releasing the key first would let a second claimant in
        // while this one still holds the lease.
        let mut conn = self.lock_pending_conn()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| FileShieldedStoreError(format!("begin claim lease release: {e}")))?;
        tx.execute(
            "DELETE FROM shielded_lifecycle_admission WHERE token = ?1 AND destructive = 0",
            rusqlite::params![token.0.as_slice()],
        )
        .map_err(|e| FileShieldedStoreError(format!("release claim lease: {e}")))?;
        tx.execute(
            "DELETE FROM shielded_one_time_claim_reservation WHERE token = ?1",
            rusqlite::params![token.0.as_slice()],
        )
        .map_err(|e| FileShieldedStoreError(format!("release claim-key reservation: {e}")))?;
        tx.commit()
            .map_err(|e| FileShieldedStoreError(format!("commit claim lease release: {e}")))?;
        Ok(())
    }

    fn begin_destructive_admission(
        &mut self,
        scope: Option<WalletId>,
        token: AdmissionToken,
        now_ms: u64,
        barrier_ms: u64,
    ) -> Result<usize, Self::Error> {
        let mut conn = self.lock_pending_conn()?;
        let tx = conn
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|e| FileShieldedStoreError(format!("begin destructive admission: {e}")))?;
        Self::reap_expired_admissions(&tx, now_ms)?;
        let scope_bytes = scope.map(|id| id.to_vec());
        // Barrier first, count second, one transaction: a claim is either
        // refused by the barrier or counted here, never both and never neither.
        tx.execute(
            "INSERT OR REPLACE INTO shielded_lifecycle_admission \
             (token, destructive, wallet_id, expires_at) VALUES (?1, 1, ?2, ?3)",
            rusqlite::params![
                token.0.as_slice(),
                scope_bytes,
                Self::as_sqlite_millis(now_ms.saturating_add(barrier_ms)),
            ],
        )
        .map_err(|e| FileShieldedStoreError(format!("insert destructive barrier: {e}")))?;
        // `?1 IS NULL` makes a store-wide purge count every wallet's claims.
        let live: i64 = tx
            .query_row(
                "SELECT COUNT(*) FROM shielded_lifecycle_admission \
                 WHERE destructive = 0 AND (?1 IS NULL OR wallet_id = ?1)",
                rusqlite::params![scope_bytes],
                |row| row.get(0),
            )
            .map_err(|e| FileShieldedStoreError(format!("count live claim leases: {e}")))?;
        tx.commit()
            .map_err(|e| FileShieldedStoreError(format!("commit destructive admission: {e}")))?;
        Ok(live.max(0) as usize)
    }

    fn end_destructive_admission(&mut self, token: AdmissionToken) -> Result<(), Self::Error> {
        let conn = self.lock_pending_conn()?;
        conn.execute(
            "DELETE FROM shielded_lifecycle_admission WHERE token = ?1 AND destructive = 1",
            rusqlite::params![token.0.as_slice()],
        )
        .map_err(|e| FileShieldedStoreError(format!("release destructive barrier: {e}")))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unique temp path for a test tree (no `tempfile` dev-dep).
    fn temp_tree_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("shielded_tree_test_{tag}_{nanos}.sqlite"))
    }

    /// A [`PendingRedrive`] survives a store reopen — record, attempt
    /// counter, AND the note reservations its nullifiers carry — and is
    /// deleted (durably) when one of its nullifiers resolves.
    #[test]
    fn redrive_roundtrip_rehydration_and_resolution() {
        let path = temp_tree_path("redrive_roundtrip");
        let id = SubwalletId::new([7u8; 32], 0);
        let redrive = PendingRedrive {
            activity_id: [1u8; 32],
            anchor: [2u8; 32],
            nullifiers: vec![[3u8; 32], [4u8; 32]],
            st_bytes: vec![0xAB; 96],
            attempts: 0,
            identity_index: None,
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id, redrive.clone()).expect("arm");
            assert_eq!(
                store
                    .bump_redrive_attempts(id, &redrive.activity_id)
                    .expect("bump"),
                1
            );
        }
        {
            // Reopen: record + attempts + reservations all rehydrated.
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            let got = store.pending_redrives(id).expect("pending_redrives");
            assert_eq!(got.len(), 1, "record survives reopen");
            assert_eq!(got[0].attempts, 1, "attempt counter persists");
            assert_eq!(got[0].st_bytes, redrive.st_bytes, "transition bytes intact");
            assert_eq!(
                store.stale_pending_spends(id).expect("stale").len(),
                2,
                "both nullifier reservations rehydrated from the record"
            );
        }
        {
            // Resolving one nullifier (release path) durably drops the row.
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 2");
            assert!(store.clear_pending(id, &[3u8; 32]).expect("clear"));
            assert!(store.pending_redrives(id).expect("redrives").is_empty());
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 3");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "deletion persisted across reopen"
            );
            assert!(
                store.stale_pending_spends(id).expect("stale").is_empty(),
                "no reservations rehydrate once the record is gone"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Purging a wallet (or all subwallets) must also delete its durable
    /// redrive rows — otherwise a Clear / unregister leaves stale rows
    /// that rehydrate ghost reservations on the next open. And
    /// `reset_commitment_tree` must NOT touch them: a redrive is
    /// broadcast state, not tree state, and a tree resync doesn't
    /// invalidate an in-flight transition.
    #[test]
    fn purge_clears_durable_redrive_rows_but_tree_reset_does_not() {
        let path = temp_tree_path("purge_redrive");
        let id_a = SubwalletId::new([0xA1; 32], 0);
        let id_b = SubwalletId::new([0xB2; 32], 0);
        let redrive = |activity: u8, nf: u8| PendingRedrive {
            activity_id: [activity; 32],
            anchor: [0x22; 32],
            nullifiers: vec![[nf; 32]],
            st_bytes: vec![0xCD; 32],
            attempts: 0,
            identity_index: None,
        };

        // purge_wallet is scoped: it drops A's rows, keeps B's.
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.arm_redrive(id_a, redrive(0x01, 0x0A)).expect("arm a");
            store.arm_redrive(id_b, redrive(0x02, 0x0B)).expect("arm b");

            // reset_commitment_tree leaves BOTH redrives intact.
            store.reset_commitment_tree().expect("reset tree");
            assert_eq!(store.pending_redrives(id_a).expect("a").len(), 1);
            assert_eq!(store.pending_redrives(id_b).expect("b").len(), 1);

            store.purge_wallet(id_a.wallet_id).expect("purge a");
            assert!(
                store.pending_redrives(id_a).expect("a").is_empty(),
                "purge_wallet dropped A's durable redrive rows"
            );
            assert_eq!(
                store.pending_redrives(id_b).expect("b").len(),
                1,
                "purge_wallet is scoped — B's rows survive"
            );
        }
        // The deletion is durable across reopen; B still rehydrates.
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert!(store.pending_redrives(id_a).expect("a").is_empty());
            assert_eq!(store.pending_redrives(id_b).expect("b").len(), 1);
        }
        // purge_all_subwallets drops everything, durably.
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 2");
            store.purge_all_subwallets().expect("purge all");
            assert!(store.pending_redrives(id_b).expect("b").is_empty());
        }
        {
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen 3");
            assert!(store.pending_redrives(id_b).expect("b").is_empty());
        }
        let _ = std::fs::remove_file(&path);
    }

    /// `mark_spent` must resolve a rehydrated redrive even for a note
    /// that is restored ALREADY spent (its transition landed in a prior
    /// session) — the durable SQLite row must be deleted too, not just
    /// the in-memory record, or it resurrects the reservation on the
    /// next open.
    #[test]
    fn mark_spent_on_restored_spent_note_clears_durable_redrive() {
        let path = temp_tree_path("mark_spent_idempotent");
        let id = SubwalletId::new([0x9; 32], 0);
        let nf = [0x3A; 32];
        let note = ShieldedNote {
            position: 0,
            cmx: [0x1; 32],
            nullifier: nf,
            block_height: 10,
            // Restored from disk ALREADY spent.
            is_spent: true,
            value: 500,
            note_data: vec![0u8; 115],
        };
        {
            let mut store = FileBackedShieldedStore::open_path(&path, 100).expect("open");
            store.save_note(id, &note).expect("save");
            store
                .arm_redrive(
                    id,
                    PendingRedrive {
                        activity_id: [0x7; 32],
                        anchor: [0x22; 32],
                        nullifiers: vec![nf],
                        st_bytes: vec![0xEF; 32],
                        attempts: 0,
                        identity_index: None,
                    },
                )
                .expect("arm");

            // Already-spent → returns false, but STILL resolves the redrive.
            let newly = store.mark_spent(id, &nf).expect("mark_spent");
            assert!(!newly, "note was already spent, so not newly spent");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "the redrive must be dropped from memory even on the already-spent path"
            );
        }
        {
            // And the durable row was deleted — nothing rehydrates.
            let store = FileBackedShieldedStore::open_path(&path, 100).expect("reopen");
            assert!(
                store.pending_redrives(id).expect("redrives").is_empty(),
                "the SQLite redrive row was mirrored-deleted, not left to rehydrate"
            );
        }
        let _ = std::fs::remove_file(&path);
    }

    /// Regression test for the "Shielded Merkle witness
    /// unavailable" spend failure (multi-wallet shared-tree bug).
    ///
    /// Root cause: the shared commitment tree previously appended
    /// commitments as `Ephemeral` unless the owning wallet's IVK
    /// recognized them in that very sync pass. With multiple
    /// wallets sharing one tree and binding at different times, a
    /// note appended before its owner bound stayed Ephemeral
    /// forever — shardtree has no retroactive marking — so the
    /// balance showed but the spend failed to build a witness.
    /// Observed on-disk symptom: every position un-witnessable
    /// (missing internal nodes at `Level(2) index 0` /
    /// `Level(1) index 2`).
    ///
    /// The fix: the shared tree marks EVERY position
    /// (`append_commitment(.., true)`); per-wallet ownership is
    /// tracked separately in the notes store. This test asserts
    /// that a fully-marked tree witnesses every position —
    /// including the rightmost (frontier) leaf whose sibling
    /// doesn't exist yet — across a persist + reload cycle (the
    /// cross-session round-trip a real wallet does between sync
    /// and spend).
    #[test]
    fn all_marked_tree_witnesses_every_position_after_reload() {
        let path = temp_tree_path("all_marked");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Mirror the real failing wallet's tree shape: 6
        // commitments, single checkpoint at the tip. The fix
        // marks ALL of them regardless of ownership.
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1; // distinct non-zero leaves
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();

        // Persist to SQLite and reopen — the wallet builds the
        // tree in one app session and witnesses it (at spend
        // time) in a later one.
        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let mut failures = Vec::new();
        for pos in 0..N {
            match store.witness(pos) {
                Ok(Some(_)) => {}
                Ok(None) => failures.push(format!("position {pos}: witness returned None")),
                Err(e) => failures.push(format!("position {pos}: {e}")),
            }
        }

        let _ = std::fs::remove_file(&path);

        assert!(
            failures.is_empty(),
            "every position in a fully-marked tree must be witnessable, but: {failures:?}"
        );
    }

    /// `tree_size()` is the append gate the multi-subwallet sync
    /// relies on to stay idempotent (it appends only positions
    /// `>= tree_size`). If the count were wrong — or didn't survive
    /// the persist + reload the wallet does between sessions — a
    /// re-fetch from a chunk boundary would double-append and
    /// corrupt the tree ("Anchor not found in the recorded anchors
    /// tree" on the next spend). This asserts the count is exact
    /// from empty, after appends, and across a reopen.
    #[test]
    fn tree_size_tracks_leaf_count_across_reload() {
        let path = temp_tree_path("tree_size");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        assert_eq!(store.tree_size().unwrap(), 0, "empty tree has size 0");

        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
            assert_eq!(
                store.tree_size().unwrap(),
                i + 1,
                "size must equal leaves appended so far"
            );
        }
        store.checkpoint_tree(N as u32).unwrap();

        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let size = store.tree_size().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            size, N,
            "tree size must survive persist + reload — the append gate \
             reads it on cold start to avoid re-appending existing leaves"
        );
    }

    /// `reset_commitment_tree()` must empty the shared tree back to
    /// zero leaves so the host's "Clear" action becomes a true cold
    /// rebuild: after a reset, `tree_size()` is 0, a fresh append
    /// starts at position 0, and the emptied state survives a
    /// persist + reload (the on-disk tables are genuinely wiped, not
    /// just the in-memory frontier). Without this, Clear rewinds the
    /// per-subwallet watermark to 0 but leaves the tree at its full
    /// size, so every re-downloaded position is gate-skipped and the
    /// "Checked" progress bar stalls.
    #[test]
    fn reset_commitment_tree_empties_and_allows_reappend_from_zero() {
        let path = temp_tree_path("reset");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Build a non-trivial tree.
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            N,
            "precondition: tree holds N leaves before reset"
        );

        // Reset wipes it back to empty.
        store.reset_commitment_tree().unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            0,
            "tree_size must be 0 immediately after reset"
        );

        // A fresh append starts at position 0 again and the count
        // climbs from there — the cold-rebuild contract Clear relies on.
        let mut cmx = [0u8; 32];
        cmx[0] = 42;
        store.append_commitment(&cmx, true).unwrap();
        assert_eq!(
            store.tree_size().unwrap(),
            1,
            "first post-reset append must land at position 0 (size 1)"
        );
        store.checkpoint_tree(1).unwrap();

        // The emptied + re-appended state must survive persist +
        // reload, proving the reset wiped the on-disk tables rather
        // than only the in-memory frontier.
        drop(store);
        let store = FileBackedShieldedStore::open_path(&path, 100).unwrap();
        let size = store.tree_size().unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            size, 1,
            "post-reset tree state (1 leaf) must survive persist + reload, \
             confirming reset cleared the SQLite tree tables"
        );
    }

    /// Durability regression guard for the "Clear did nothing after a
    /// force-stop" bug: `reset_commitment_tree` must land the emptied
    /// tables in the MAIN database file, not merely in the `-wal` file of
    /// the store's own connections.
    ///
    /// The store runs `synchronous=NORMAL` in WAL mode and keeps multiple
    /// connections open, so SQLite's last-connection-close auto-checkpoint
    /// never fires on the transient reset connection — without an explicit
    /// checkpoint the DELETEs live only in the WAL. On Android the Clear
    /// button is routinely followed by a force-stop (SIGKILL, no graceful
    /// close, no checkpoint), which discards those WAL frames and reopens
    /// the OLD full tree (the 771/771 symptom).
    ///
    /// This test proves the fix WITHOUT dropping the store (a graceful drop
    /// would checkpoint and mask the bug — exactly what the prior test's
    /// `drop(store)` did): it opens an INDEPENDENT read-only connection that
    /// deliberately does NOT attach the `-wal` (`?immutable=1`), so it can
    /// only see rows already written to the main db file. If the reset left
    /// the shard rows in the WAL, this connection would still see the old
    /// leaves; seeing zero proves the checkpoint flushed them to the main db,
    /// where they survive any later process death.
    #[test]
    fn reset_commitment_tree_flushes_to_main_db_file_not_just_wal() {
        let path = temp_tree_path("reset_durable");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        // Build a non-trivial tree and checkpoint it (durably, via the
        // normal append path).
        const N: u64 = 6;
        for i in 0..N {
            let mut cmx = [0u8; 32];
            cmx[0] = (i as u8) + 1;
            store.append_commitment(&cmx, true).unwrap();
        }
        store.checkpoint_tree(N as u32).unwrap();
        assert_eq!(store.tree_size().unwrap(), N);

        // Reset. The store's connections stay open (mirroring a live
        // process that hasn't been force-stopped yet).
        store.reset_commitment_tree().unwrap();
        assert_eq!(store.tree_size().unwrap(), 0);

        // Independent immutable connection: reads ONLY the main .sqlite
        // file, ignoring any `-wal`. `immutable=1` tells SQLite the file
        // won't change and there is no live WAL to consult, so a shard row
        // visible here is one the checkpoint flushed into the main db.
        let uri = format!("file:{}?immutable=1", path.display());
        let main_only = rusqlite::Connection::open_with_flags(
            &uri,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .expect("open main-db-only connection");
        let shard_rows: i64 = main_only
            .query_row("SELECT COUNT(*) FROM commitment_tree_shards", [], |r| {
                r.get(0)
            })
            .expect("count shard rows in main db");
        drop(main_only);
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            shard_rows, 0,
            "reset must checkpoint the emptied tables into the MAIN db file; \
             a non-zero count means the DELETEs sit only in the WAL and a \
             force-stop before checkpoint would resurrect the old tree"
        );
    }

    /// Reproduces the shielded **withdrawal-never-lands** root cause (TestFlight
    /// report B): the wallet builds a spend against its depth-0 (current) tree
    /// root, but that root is a *Platform-recorded* anchor only when the tree
    /// sits exactly on a block boundary.
    ///
    /// - The spend anchor is `witness(pos, 0).root(cmx)`, which equals
    ///   `tree_anchor()` (both depth-0; see the comment on `witness`). This test
    ///   asserts that equality directly.
    /// - The wallet syncs commitments by index-chunk (`CHUNK_SIZE = 2048` in
    ///   `sync.rs`), **not** by block, so its tree routinely stops mid-block.
    /// - drive records **one anchor per block** (`record_anchor_if_changed` at
    ///   block-processing-end) and `validate_anchor_exists` rejects any anchor
    ///   it never recorded (`InvalidAnchorError`).
    ///
    /// So a mid-block depth-0 anchor is rejected every attempt — repeatable,
    /// never lands, funds untouched. The team already names this failure at the
    /// `tree_size` test above ("Anchor not found in the recorded anchors").
    #[test]
    fn depth0_spend_anchor_mid_block_is_not_a_recorded_block_boundary_anchor() {
        use grovedb_commitment_tree::ExtractedNoteCommitment;

        let path = temp_tree_path("anchor_midblock");
        let mut store = FileBackedShieldedStore::open_path(&path, 100).unwrap();

        let cmx = |b: u8| {
            let mut c = [0u8; 32];
            c[0] = b;
            c
        };

        // Two blocks of commitments. drive records ONE anchor per block, at
        // block-processing-end (after ALL of that block's commitments):
        //   block 1 = commitments 1..=3  -> recorded anchor at tree size 3
        //   block 2 = commitments 4..=6  -> recorded anchor at tree size 6
        for b in 1..=3u8 {
            store.append_commitment(&cmx(b), true).unwrap();
        }
        store.checkpoint_tree(3).unwrap();
        let recorded_after_block1 = store.tree_anchor().unwrap();

        // The index-chunk sync appends block 2's commitments incrementally; a
        // chunk/stream boundary that lands mid-block (the common case — a
        // 2048-leaf chunk rarely ends on a block boundary) leaves the wallet at
        // tree size 4, and it checkpoints there (sync.rs checkpoints at the
        // post-append leaf count). Its depth-0 anchor is now the root at size 4
        // — a state drive never recorded.
        store.append_commitment(&cmx(4), true).unwrap();
        store.checkpoint_tree(4).unwrap();
        let wallet_depth0_mid_block = store.tree_anchor().unwrap();

        // The spend path uses exactly this anchor: `extract_spends_and_anchor`
        // builds it as `witness(pos, 0).root(cmx)`. Pin that it equals the
        // mid-block `tree_anchor()`.
        let cmx0 = ExtractedNoteCommitment::from_bytes(&cmx(1))
            .into_option()
            .expect("valid cmx");
        let spend_anchor = store
            .witness(0)
            .unwrap()
            .expect("witness for marked position 0")
            .root(cmx0)
            .to_bytes();
        assert_eq!(
            spend_anchor, wallet_depth0_mid_block,
            "the spend anchor (depth-0 witness root) must equal the mid-block tree_anchor"
        );

        // Finish block 2. drive records the anchor at tree size 6.
        store.append_commitment(&cmx(5), true).unwrap();
        store.append_commitment(&cmx(6), true).unwrap();
        store.checkpoint_tree(6).unwrap();
        let recorded_after_block2 = store.tree_anchor().unwrap();

        let _ = std::fs::remove_file(&path);

        // drive's recorded anchor set is {block1, block2}. The wallet's mid-block
        // spend anchor is neither -> `validate_anchor_exists` rejects it with
        // InvalidAnchorError, and the withdrawal never lands.
        assert_ne!(
            wallet_depth0_mid_block, recorded_after_block1,
            "mid-block spend anchor must differ from block 1's recorded anchor"
        );
        assert_ne!(
            wallet_depth0_mid_block, recorded_after_block2,
            "mid-block spend anchor must differ from block 2's recorded anchor"
        );
        assert_ne!(
            recorded_after_block1, recorded_after_block2,
            "the two block-boundary anchors differ (the tree grew), so drive's \
             recorded set is exactly these two and the mid-block anchor is outside it"
        );
    }

    // ── Lifecycle admission (#4313) ────────────────────────────────────
    //
    // The fence's whole point is that it works between store INSTANCES, which
    // is what a coordinator-local `tokio::sync::Mutex` cannot do: every test
    // below that matters opens two `FileBackedShieldedStore`s on the same file,
    // exactly as two `NetworkShieldedCoordinator`s (or two processes) would.

    /// The reserved account claim records live under, mirrored here so these
    /// tests exercise the real key space.
    const CLAIM_ACCOUNT: u32 = u32::MAX;

    fn admission_record(activity: u8) -> PendingRedrive {
        PendingRedrive {
            activity_id: [activity; 32],
            anchor: [0x0A; 32],
            nullifiers: vec![[0x0B; 32]],
            st_bytes: vec![0xCD; 64],
            attempts: 0,
            identity_index: None,
        }
    }

    /// A destructive barrier taken by one store instance REFUSES a claim
    /// admitted through a different instance on the same file.
    ///
    /// This is the interleaving the coordinator-local guard cannot cover: the
    /// two stores share the SQLite file and nothing else.
    #[test]
    fn a_barrier_in_one_store_instance_refuses_a_claim_in_another() {
        let path = temp_tree_path("admission_barrier_blocks");
        let wallet_id: WalletId = [0x21; 32];
        let mut purger = FileBackedShieldedStore::open_path(&path, 8).expect("store a");
        let mut claimer = FileBackedShieldedStore::open_path(&path, 8).expect("store b");
        let now = 1_000_000;

        let barrier = AdmissionToken::new();
        assert_eq!(
            purger
                .begin_destructive_admission(Some(wallet_id), barrier, now, 60_000)
                .expect("barrier"),
            0,
            "no claim is in flight yet"
        );

        assert!(
            !claimer
                .begin_claim_admission(wallet_id, AdmissionToken::new(), now, 60_000)
                .expect("claim admission"),
            "a claim must be refused while another instance holds destructive admission"
        );

        // Releasing the barrier lets claims back in.
        purger
            .end_destructive_admission(barrier)
            .expect("release barrier");
        assert!(claimer
            .begin_claim_admission(wallet_id, AdmissionToken::new(), now, 60_000)
            .expect("claim admission"));

        drop((purger, claimer));
        let _ = std::fs::remove_file(&path);
    }

    /// The other order: a claim lease taken through one instance is COUNTED by
    /// a destructive admission taken through another, so the purge waits
    /// instead of deleting the record out from under an in-flight claim.
    #[test]
    fn a_live_claim_in_one_store_instance_is_counted_by_another() {
        let path = temp_tree_path("admission_lease_counted");
        let wallet_id: WalletId = [0x22; 32];
        let mut claimer = FileBackedShieldedStore::open_path(&path, 8).expect("store a");
        let mut purger = FileBackedShieldedStore::open_path(&path, 8).expect("store b");
        let now = 1_000_000;

        let lease = AdmissionToken::new();
        assert!(claimer
            .begin_claim_admission(wallet_id, lease, now, 60_000)
            .expect("claim admission"));

        assert_eq!(
            purger
                .begin_destructive_admission(Some(wallet_id), AdmissionToken::new(), now, 60_000)
                .expect("barrier"),
            1,
            "the purge must see the other instance's in-flight claim and wait"
        );

        // Once the claim releases, the next poll drains.
        claimer.end_claim_admission(lease).expect("release lease");
        assert_eq!(
            purger
                .begin_destructive_admission(Some(wallet_id), AdmissionToken::new(), now, 60_000)
                .expect("barrier refresh"),
            0
        );

        drop((claimer, purger));
        let _ = std::fs::remove_file(&path);
    }

    /// Arming is admitted in the SAME step as the lease re-check, and refuses —
    /// writing nothing — once the lease is gone. This is the gap a separate
    /// "check, then write" would leave open for a purge to slot into.
    #[test]
    fn arming_refuses_and_writes_nothing_once_the_lease_is_gone() {
        let path = temp_tree_path("admission_arm_refuses");
        let wallet_id: WalletId = [0x23; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");
        let now = 1_000_000;

        let lease = AdmissionToken::new();
        assert!(store
            .begin_claim_admission(wallet_id, lease, now, 60_000)
            .expect("claim admission"));
        assert!(
            store
                .arm_redrive_under_claim(id, admission_record(0x01), lease, now, 60_000)
                .expect("arm under a live lease"),
            "a live lease must admit the record write"
        );
        assert_eq!(store.pending_redrives(id).expect("records").len(), 1);

        // Lease released (or expired): a further arm must be refused outright.
        store.end_claim_admission(lease).expect("release lease");
        assert!(
            !store
                .arm_redrive_under_claim(id, admission_record(0x02), lease, now, 60_000)
                .expect("arm without a lease"),
            "arming without a live lease must be refused"
        );
        let records = store.pending_redrives(id).expect("records");
        assert_eq!(
            records.len(),
            1,
            "the refused arm must not have written anything"
        );
        assert_eq!(records[0].activity_id, [0x01; 32]);

        // …and the refusal is durable, not just in-memory: a cold reopen sees
        // only the admitted record.
        drop(store);
        let reopened = FileBackedShieldedStore::open_path(&path, 8).expect("reopen");
        assert_eq!(reopened.pending_redrives(id).expect("records").len(), 1);
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    /// An expired lease is a LIVENESS backstop, not a hole: it never removes a
    /// live claim, it only stops a holder that died from blocking wallet
    /// removal forever.
    #[test]
    fn an_expired_lease_stops_blocking_the_purge() {
        let path = temp_tree_path("admission_lease_expiry");
        let wallet_id: WalletId = [0x24; 32];
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");
        let now = 1_000_000;

        // A lease that is already dead by the time the purge looks.
        assert!(store
            .begin_claim_admission(wallet_id, AdmissionToken::new(), now, 10)
            .expect("claim admission"));
        assert_eq!(
            store
                .begin_destructive_admission(
                    Some(wallet_id),
                    AdmissionToken::new(),
                    now + 5,
                    60_000
                )
                .expect("barrier while the lease is live"),
            1,
            "a lease that has not expired yet must still block"
        );
        assert_eq!(
            store
                .begin_destructive_admission(
                    Some(wallet_id),
                    AdmissionToken::new(),
                    now + 5_000,
                    60_000
                )
                .expect("barrier after the lease expired"),
            0,
            "an expired lease must be reaped so wallet removal can proceed"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// Scope: `purge_wallet`'s barrier is wallet-scoped and must not refuse
    /// another wallet's claim, while `clear`'s store-wide barrier refuses both.
    #[test]
    fn barrier_scope_matches_the_lifecycle_operation() {
        let path = temp_tree_path("admission_scope");
        let mine: WalletId = [0x25; 32];
        let theirs: WalletId = [0x26; 32];
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");
        let now = 1_000_000;

        let scoped = AdmissionToken::new();
        store
            .begin_destructive_admission(Some(mine), scoped, now, 60_000)
            .expect("scoped barrier");
        assert!(
            !store
                .begin_claim_admission(mine, AdmissionToken::new(), now, 60_000)
                .expect("own-wallet claim"),
            "a wallet-scoped barrier must refuse that wallet's claims"
        );
        let other_lease = AdmissionToken::new();
        assert!(
            store
                .begin_claim_admission(theirs, other_lease, now, 60_000)
                .expect("other-wallet claim"),
            "a wallet-scoped barrier must not refuse an unrelated wallet's claim"
        );
        store
            .end_destructive_admission(scoped)
            .expect("release scoped");
        store
            .end_claim_admission(other_lease)
            .expect("release other lease");

        // Store-wide (`clear`) refuses everything…
        let wide = AdmissionToken::new();
        store
            .begin_destructive_admission(None, wide, now, 60_000)
            .expect("store-wide barrier");
        assert!(!store
            .begin_claim_admission(mine, AdmissionToken::new(), now, 60_000)
            .expect("claim under a store-wide barrier"));
        assert!(!store
            .begin_claim_admission(theirs, AdmissionToken::new(), now, 60_000)
            .expect("claim under a store-wide barrier"));
        store.end_destructive_admission(wide).expect("release wide");

        // …and counts every wallet's claims when deciding whether to wait.
        assert!(store
            .begin_claim_admission(mine, AdmissionToken::new(), now, 60_000)
            .expect("claim"));
        assert!(store
            .begin_claim_admission(theirs, AdmissionToken::new(), now, 60_000)
            .expect("claim"));
        assert_eq!(
            store
                .begin_destructive_admission(None, AdmissionToken::new(), now, 60_000)
                .expect("store-wide barrier"),
            2,
            "clear() must wait for every wallet's in-flight claims"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// THE FINDING, end to end at the store: an armed claim record is NOT
    /// deleted by a concurrent purge, because the purge cannot get past the
    /// live lease — even though the purge runs through a different store
    /// instance, which is precisely where the coordinator-local guard failed.
    ///
    /// The second half shows the fence is a fence and not a lock-out: once the
    /// claim releases, the purge is admitted and the record goes with it, so
    /// `remove_wallet`'s full-wipe contract is unchanged.
    #[test]
    fn a_purge_cannot_delete_a_record_while_the_claim_that_armed_it_is_live() {
        let path = temp_tree_path("admission_end_to_end");
        let wallet_id: WalletId = [0x27; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut claimer = FileBackedShieldedStore::open_path(&path, 8).expect("claimer store");
        let mut purger = FileBackedShieldedStore::open_path(&path, 8).expect("purger store");
        let now = 1_000_000;

        let lease = AdmissionToken::new();
        assert!(claimer
            .begin_claim_admission(wallet_id, lease, now, 60_000)
            .expect("claim admission"));
        assert!(claimer
            .arm_redrive_under_claim(id, admission_record(0x09), lease, now, 60_000)
            .expect("arm"));

        // The purge's own admission tells it to wait — so it never calls
        // `purge_wallet`, and the record survives.
        assert_eq!(
            purger
                .begin_destructive_admission(Some(wallet_id), AdmissionToken::new(), now, 60_000)
                .expect("barrier"),
            1,
            "the purge must be told to wait, not cleared to delete"
        );
        assert_eq!(
            claimer.pending_redrives(id).expect("records").len(),
            1,
            "the in-flight claim's recovery record must still be there"
        );

        // Claim done: the purge drains and the full wipe proceeds as before.
        claimer.end_claim_admission(lease).expect("release lease");
        assert_eq!(
            purger
                .begin_destructive_admission(Some(wallet_id), AdmissionToken::new(), now, 60_000)
                .expect("barrier refresh"),
            0
        );
        purger.purge_wallet(wallet_id).expect("purge");
        drop((claimer, purger));

        let reopened = FileBackedShieldedStore::open_path(&path, 8).expect("reopen");
        assert!(
            reopened.pending_redrives(id).expect("records").is_empty(),
            "once admitted, the purge is still a FULL wipe — no reserved account is exempted"
        );
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    // ── Claim-record identity slot (#4313 5d4d6efa) ────────────────────

    /// A database created before `shielded_pending_spends.identity_index`
    /// existed must open, gain the column, and keep its rows — with the slot
    /// reading `None` rather than a back-filled guess a resume would then
    /// enforce. This store versions its schema by `CREATE TABLE IF NOT EXISTS`,
    /// so a `PRAGMA table_info` probe plus `ALTER TABLE` is the matching
    /// idempotent form; the second open below proves it does not re-run.
    #[test]
    fn a_pre_migration_database_gains_the_identity_index_column() {
        let path = temp_tree_path("pending_spends_migration");
        let id = SubwalletId::new([0x51; 32], CLAIM_ACCOUNT);

        // Build the OLD schema by hand — no identity_index column — and seed a
        // record through it, exactly as a shipped build would have left it.
        {
            let conn = Connection::open(&path).expect("raw open");
            conn.execute(
                "CREATE TABLE shielded_pending_spends (
                    wallet_id     BLOB    NOT NULL,
                    account_index INTEGER NOT NULL,
                    activity_id   BLOB    NOT NULL,
                    anchor        BLOB    NOT NULL,
                    nullifiers    BLOB    NOT NULL,
                    st_bytes      BLOB    NOT NULL,
                    attempts      INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (wallet_id, account_index, activity_id)
                )",
                [],
            )
            .expect("old table");
            conn.execute(
                "INSERT INTO shielded_pending_spends \
                 (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    [0x99u8; 32].as_slice(),
                    [0x0Au8; 32].as_slice(),
                    [0x0Bu8; 32].as_slice(),
                    vec![0xCDu8; 64],
                ],
            )
            .expect("old row");
        }

        let store = FileBackedShieldedStore::open_path(&path, 8).expect("migrating open");
        let records = store.pending_redrives(id).expect("records");
        assert_eq!(records.len(), 1, "the pre-migration row must survive");
        assert_eq!(records[0].activity_id, [0x99; 32]);
        assert_eq!(
            records[0].identity_index, None,
            "a record that predates the column knows no slot, and must say so"
        );
        drop(store);

        // Idempotent: opening again must not try to add the column twice.
        let reopened = FileBackedShieldedStore::open_path(&path, 8).expect("second open");
        assert_eq!(reopened.pending_redrives(id).expect("records").len(), 1);
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    /// A claim record's slot must round-trip through SQLite, not merely live in
    /// the in-memory mirror: recovering it after a process restart is the only
    /// reason to persist it at all.
    #[test]
    fn a_claim_records_identity_index_survives_a_reopen() {
        let path = temp_tree_path("pending_spends_slot_roundtrip");
        let wallet_id: WalletId = [0x52; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");

        let mut record = admission_record(0x77);
        record.identity_index = Some(9);
        store.arm_redrive(id, record).expect("arm");
        drop(store);

        let reopened = FileBackedShieldedStore::open_path(&path, 8).expect("reopen");
        let records = reopened.pending_redrives(id).expect("records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].identity_index, Some(9));
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    // ── Per-invitation claim-key reservation (#4313 cr-9d0e1a44) ───────
    //
    // A claim LEASE is per-wallet and admits both claimants of one invitation.
    // These tests cover the reservation that is per-INVITATION, and they open
    // two store instances on the same file for the same reason the tests above
    // do: that is precisely the interleaving the coordinator's per-FVK mutex
    // cannot see.

    /// THE BUG: two coordinators (or two processes) on one SQLite file both got
    /// admitted for the same invitation, built transitions with DIFFERENT
    /// padded identity ids, and the second `arm_redrive_under_claim` —
    /// `INSERT OR REPLACE` — overwrote the first's byte-exact recovery row
    /// while the first's transition was already on the wire, stranding that
    /// identity forever.
    ///
    /// Exactly one claimant may acquire the key; the loser is told who holds
    /// it, and the storage layer refuses its arm outright so the winner's row
    /// survives byte-for-byte.
    #[test]
    fn two_store_instances_cannot_both_claim_one_invitation() {
        let path = temp_tree_path("claim_key_race");
        let wallet_id: WalletId = [0x31; 32];
        let claim_key = [0xC1; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut first = FileBackedShieldedStore::open_path(&path, 8).expect("store a");
        let mut second = FileBackedShieldedStore::open_path(&path, 8).expect("store b");
        let now = 1_000_000;

        // Both are admitted by the per-WALLET lease — that is the point: the
        // lease is not, and was never, mutual exclusion between claimants.
        let winner = AdmissionToken::new();
        let loser = AdmissionToken::new();
        assert!(first
            .begin_claim_admission(wallet_id, winner, now, 60_000)
            .expect("first lease"));
        assert!(
            second
                .begin_claim_admission(wallet_id, loser, now, 60_000)
                .expect("second lease"),
            "the per-wallet lease admits both claimants; only the claim-key \
             reservation separates them"
        );

        // The key, however, admits exactly one.
        assert_eq!(
            first
                .reserve_one_time_claim_key(id, claim_key, winner, now, 60_000)
                .expect("first reservation")
                .reservation,
            ClaimKeyReservation::Acquired
        );
        assert_eq!(
            second
                .reserve_one_time_claim_key(id, claim_key, loser, now, 60_000)
                .expect("second reservation")
                .reservation,
            ClaimKeyReservation::Held {
                holder: winner,
                expires_at: now + 60_000,
            },
            "the loser must be handed the DURABLE row, not a fresh one of its own"
        );

        // The winner arms its byte-exact recovery record.
        let mut winning_record = admission_record(0xC1);
        winning_record.activity_id = claim_key;
        winning_record.st_bytes = vec![0xAA; 96];
        assert!(first
            .arm_redrive_under_claim(id, winning_record.clone(), winner, now, 60_000)
            .expect("winner arms"));

        // The loser's arm — the INSERT OR REPLACE that used to clobber — is
        // refused at the storage layer, even though its OWN lease is live.
        let mut losing_record = admission_record(0xC2);
        losing_record.activity_id = claim_key;
        losing_record.st_bytes = vec![0xBB; 96];
        assert!(
            !second
                .arm_redrive_under_claim(id, losing_record, loser, now, 60_000)
                .expect("loser arms"),
            "a claimant that does not hold the key must not be able to write this record"
        );

        // The winner's record is intact, byte-for-byte, on a cold reopen.
        drop((first, second));
        let reopened = FileBackedShieldedStore::open_path(&path, 8).expect("reopen");
        let records = reopened.pending_redrives(id).expect("records");
        assert_eq!(
            records.len(),
            1,
            "exactly one claim record for one invitation"
        );
        assert_eq!(records[0].activity_id, claim_key);
        assert_eq!(
            records[0].st_bytes,
            vec![0xAA; 96],
            "the winner's byte-exact transition must survive the loser's attempt"
        );
        drop(reopened);
        let _ = std::fs::remove_file(&path);
    }

    /// The reservation is bound to its lease for its whole life: released with
    /// it, re-stamped with it, and otherwise reaped by expiry so a claimant
    /// that died cannot hold an invitation hostage.
    #[test]
    fn a_claim_key_reservation_lives_and_dies_with_its_lease() {
        let path = temp_tree_path("claim_key_lifetime");
        let wallet_id: WalletId = [0x32; 32];
        let claim_key = [0xC3; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");
        let now = 1_000_000;

        let first = AdmissionToken::new();
        assert!(store
            .begin_claim_admission(wallet_id, first, now, 60_000)
            .expect("lease"));
        assert_eq!(
            store
                .reserve_one_time_claim_key(id, claim_key, first, now, 60_000)
                .expect("reserve")
                .reservation,
            ClaimKeyReservation::Acquired
        );
        // Re-entry by the SAME token is idempotent, never a self-lockout.
        assert_eq!(
            store
                .reserve_one_time_claim_key(id, claim_key, first, now + 1, 60_000)
                .expect("re-enter")
                .reservation,
            ClaimKeyReservation::Acquired
        );

        // Renewing the lease carries the reservation with it, so a long claim
        // cannot lose its invitation to expiry while its lease is kept alive.
        assert!(store
            .renew_claim_admission(first, now + 30_000, 60_000)
            .expect("renew"));
        let second = AdmissionToken::new();
        assert!(store
            .begin_claim_admission(wallet_id, second, now + 70_000, 60_000)
            .expect("second lease"));
        assert!(
            !store
                .reserve_one_time_claim_key(id, claim_key, second, now + 70_000, 60_000)
                .expect("contend after renewal")
                .is_acquired(),
            "the renewed reservation must still be held past the ORIGINAL expiry"
        );

        // Releasing the lease releases the key in the same step — the next
        // claimant of this invitation must not wait out a full lease period.
        store.end_claim_admission(first).expect("release");
        assert_eq!(
            store
                .reserve_one_time_claim_key(id, claim_key, second, now + 70_000, 60_000)
                .expect("reserve after release")
                .reservation,
            ClaimKeyReservation::Acquired
        );

        // And a holder that dies without releasing ages out rather than
        // stranding the invitation forever.
        let third = AdmissionToken::new();
        assert_eq!(
            store
                .reserve_one_time_claim_key(id, claim_key, third, now + 200_000, 60_000)
                .expect("reserve after expiry")
                .reservation,
            ClaimKeyReservation::Acquired,
            "an expired reservation must be reaped"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// The claim-key gate must not touch ordinary spend redrives, which never
    /// take a reservation: a different invitation's live hold is irrelevant to
    /// them, and to each other.
    #[test]
    fn the_claim_key_gate_leaves_unreserved_redrives_alone() {
        let path = temp_tree_path("claim_key_gate_scope");
        let wallet_id: WalletId = [0x33; 32];
        let id = SubwalletId::new(wallet_id, 0);
        let mut store = FileBackedShieldedStore::open_path(&path, 8).expect("store");
        let now = 1_000_000;

        let holder = AdmissionToken::new();
        let other = AdmissionToken::new();
        assert!(store
            .begin_claim_admission(wallet_id, holder, now, 60_000)
            .expect("holder lease"));
        assert!(store
            .begin_claim_admission(wallet_id, other, now, 60_000)
            .expect("other lease"));
        store
            .reserve_one_time_claim_key(
                SubwalletId::new(wallet_id, CLAIM_ACCOUNT),
                [0xC4; 32],
                holder,
                now,
                60_000,
            )
            .expect("reserve one invitation");

        // A redrive under a DIFFERENT activity id is unaffected by that hold.
        assert!(
            store
                .arm_redrive_under_claim(id, admission_record(0xD1), other, now, 60_000)
                .expect("unrelated redrive"),
            "an unreserved activity id must still arm normally"
        );

        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// A pending-claim row armed by store A must come back to store B the
    /// moment B acquires the released reservation — read from SQLITE inside
    /// the reservation's own transaction, never from B's startup-hydrated
    /// mirror (#4313 review finding r3767229122).
    ///
    /// This is the exact sequence the reviewer described. A arms a claim,
    /// returns `ShieldedBroadcastUnconfirmed` (record kept, reservation
    /// released) and B takes the freed key. Before the fix B's mirror — loaded
    /// once when B opened, which was BEFORE A armed anything — reported no
    /// record, so B built a SECOND transition with a different padded identity
    /// id and replaced A's row. If A's transition had executed, its randomized
    /// id was then unrecoverable forever.
    #[test]
    fn a_peer_stores_pending_claim_row_comes_back_with_the_reservation() {
        let path = temp_tree_path("claim_row_handover");
        let wallet_id: WalletId = [0x34; 32];
        let claim_key = [0xC7; 32];
        let id = SubwalletId::new(wallet_id, CLAIM_ACCOUNT);
        let mut a = FileBackedShieldedStore::open_path(&path, 8).expect("store a");
        // B opens BEFORE A arms anything: its mirror is hydrated now and never
        // again, which is precisely the staleness this test exists to defeat.
        let mut b = FileBackedShieldedStore::open_path(&path, 8).expect("store b");
        let now = 1_000_000;

        // ---- A: lease, reserve, arm, release ----
        let a_token = AdmissionToken::new();
        assert!(a
            .begin_claim_admission(wallet_id, a_token, now, 60_000)
            .expect("a lease"));
        let a_out = a
            .reserve_one_time_claim_key(id, claim_key, a_token, now, 60_000)
            .expect("a reserve");
        assert_eq!(a_out.reservation, ClaimKeyReservation::Acquired);
        assert!(
            a_out.pending.is_none(),
            "a fresh invitation has no record to resume"
        );

        let mut record = admission_record(0xC7);
        record.activity_id = claim_key;
        record.st_bytes = vec![0xA7; 128];
        record.identity_index = Some(9);
        assert!(a
            .arm_redrive_under_claim(id, record.clone(), a_token, now, 60_000)
            .expect("a arms"));
        // The ShieldedBroadcastUnconfirmed shape: the RECORD is deliberately
        // kept (its retry needs the declared id) while the lease and its
        // reservation are released.
        a.end_claim_admission(a_token).expect("a releases");

        // ---- The precondition: B's mirror is blind to A's row ----
        assert!(
            b.pending_redrives(id).expect("b mirror").is_empty(),
            "precondition: B's startup-hydrated mirror cannot see A's row — if this \
             ever starts passing by itself the mirror changed, and the assertion \
             below is no longer testing what it claims"
        );

        // ---- B: acquires the freed key and MUST be handed A's row ----
        let b_token = AdmissionToken::new();
        assert!(b
            .begin_claim_admission(wallet_id, b_token, now + 1, 60_000)
            .expect("b lease"));
        let b_out = b
            .reserve_one_time_claim_key(id, claim_key, b_token, now + 1, 60_000)
            .expect("b reserve");
        assert_eq!(
            b_out.reservation,
            ClaimKeyReservation::Acquired,
            "A released, so the key is B's to take"
        );
        let resumed = b_out
            .pending
            .expect("B must RESUME A's durable row, not find None and arm a fresh one");
        assert_eq!(
            resumed.st_bytes,
            vec![0xA7; 128],
            "A's byte-exact transition — the only handle on its padded identity id — \
             must survive into B's resume"
        );
        assert_eq!(resumed.identity_index, Some(9));
        assert_eq!(resumed.activity_id, claim_key);
        assert_eq!(resumed.anchor, record.anchor);
        assert_eq!(resumed.nullifiers, record.nullifiers);

        // The handover also folds the row into B's mirror, so the rest of B's
        // claim (attempt bumps, finalize/clear) agrees with disk.
        assert_eq!(
            b.pending_redrives(id).expect("b mirror after").len(),
            1,
            "the resumed row must be visible to B's own later reads"
        );

        drop((a, b));
        let _ = std::fs::remove_file(&path);
    }

    /// The recovery connection runs at `synchronous=FULL`; the commitment
    /// tree's stays at `NORMAL` (#4313 review finding file_store.rs:107).
    ///
    /// The asymmetry is the point. A claim record's `st_bytes` carry a
    /// randomized padded identity id that exists nowhere else, and it is
    /// broadcast immediately after the commit — so a commit that returns
    /// before the WAL is on disk can lose an identity permanently. Every row
    /// in the tree, by contrast, is chain-authenticated and rebuildable by
    /// re-running sync, and fsync'ing per `append_commitment` is what made a
    /// 1M-leaf build take minutes instead of seconds.
    #[test]
    fn the_recovery_connection_is_fsync_durable_and_the_tree_connection_is_not() {
        let path = temp_tree_path("pending_conn_sync_level");
        let store = FileBackedShieldedStore::open_path(&path, 8).expect("store");

        // 2 == FULL in SQLite's `synchronous` encoding (0 OFF, 1 NORMAL,
        // 2 FULL, 3 EXTRA).
        let recovery: i32 = store
            .pending_conn
            .lock()
            .expect("pending_conn mutex")
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .expect("read recovery synchronous");
        assert_eq!(
            recovery, 2,
            "the claim-recovery connection must be synchronous=FULL: its row is \
             unreconstructable once the transition it describes is on the wire"
        );

        let tree = FileBackedShieldedStore::open_tuned_connection(&path).expect("tree conn");
        let tree_level: i32 = tree
            .pragma_query_value(None, "synchronous", |row| row.get(0))
            .expect("read tree synchronous");
        assert_eq!(
            tree_level, 1,
            "the commitment-tree connection must stay NORMAL — paying an fsync per \
             appended cmx is the cost this split exists to avoid"
        );

        drop(tree);
        drop(store);
        let _ = std::fs::remove_file(&path);
    }

    /// Two openers racing the `identity_index` migration must both succeed
    /// (#4313 review finding file_store.rs:206).
    ///
    /// Probe-then-ALTER used to be two steps, so two processes opening one file
    /// could both read "absent", and the loser's `open_path` failed outright
    /// with `duplicate column name`. Both guards are pinned here: the
    /// sequential double-open (which the `BEGIN IMMEDIATE` orders), and the
    /// classification of the duplicate-column rejection as benign.
    #[test]
    fn the_identity_index_migration_tolerates_a_racing_opener() {
        let path = temp_tree_path("identity_index_migration_race");

        // A pre-#4313 database: the old table shape, with no identity_index.
        {
            let conn = rusqlite::Connection::open(&path).expect("legacy conn");
            conn.execute(
                "CREATE TABLE shielded_pending_spends (
                    wallet_id      BLOB    NOT NULL,
                    account_index  INTEGER NOT NULL,
                    activity_id    BLOB    NOT NULL,
                    anchor         BLOB    NOT NULL,
                    nullifiers     BLOB    NOT NULL,
                    st_bytes       BLOB    NOT NULL,
                    attempts       INTEGER NOT NULL DEFAULT 0,
                    PRIMARY KEY (wallet_id, account_index, activity_id)
                )",
                [],
            )
            .expect("legacy table");
        }

        let first = FileBackedShieldedStore::open_path(&path, 8).expect("first open migrates");
        let second = FileBackedShieldedStore::open_path(&path, 8)
            .expect("a second open must not fail on the column the first one added");
        drop((first, second));

        // The tolerated error is genuinely recognised — the belt to the
        // BEGIN IMMEDIATE braces. Matched on SQLite's real message rather than
        // a hand-written string.
        let conn = rusqlite::Connection::open(&path).expect("probe conn");
        let err = conn
            .execute(
                "ALTER TABLE shielded_pending_spends ADD COLUMN identity_index INTEGER",
                [],
            )
            .expect_err("the column exists by now, so this must be rejected");
        assert!(
            FileBackedShieldedStore::is_duplicate_column(&err),
            "the duplicate-column rejection must be classified benign, got: {err}"
        );
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }
}
