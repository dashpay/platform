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
use std::sync::Mutex;

use grovedb_commitment_tree::{ClientPersistentCommitmentTree, Position, Retention};

use super::store::{
    PendingRedrive, ShieldedNote, ShieldedOutgoingNote, ShieldedStore, StalePendingSpend,
    SubwalletId, SubwalletState,
};
use crate::wallet::platform_wallet::WalletId;

/// Error type for [`FileBackedShieldedStore`].
#[derive(Debug)]
pub struct FileShieldedStoreError(pub String);

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
        let pending_conn = Self::open_tuned_connection(&path)?;
        pending_conn
            .execute(
                "CREATE TABLE IF NOT EXISTS shielded_pending_spends (
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
            .map_err(|e| FileShieldedStoreError(format!("create pending_spends table: {e}")))?;
        let mut store = Self {
            tree: Mutex::new(tree),
            path,
            max_checkpoints,
            subwallets: BTreeMap::new(),
            pending_conn: Mutex::new(pending_conn),
        };
        store.rehydrate_pending_spends()?;
        Ok(store)
    }

    /// Reload every persisted [`PendingRedrive`] into the in-memory
    /// per-subwallet state, re-arming both the redrive record and the
    /// note reservations its nullifiers carry — an unconfirmed
    /// broadcast therefore keeps its notes reserved (and its re-drive
    /// alive) across restarts. Corrupt rows are dropped with a warning
    /// rather than failing the open.
    fn rehydrate_pending_spends(&mut self) -> Result<(), FileShieldedStoreError> {
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
        let mut stmt = conn
            .prepare(
                "SELECT wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, \
                 attempts FROM shielded_pending_spends",
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
                ))
            })
            .map_err(|e| FileShieldedStoreError(format!("query rehydrate: {e}")))?;
        for row in rows {
            let (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts) =
                row.map_err(|e| FileShieldedStoreError(format!("read rehydrate row: {e}")))?;
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
            let sw = self.subwallets.entry(id).or_default();
            for n in &nullifiers {
                sw.mark_pending(n);
                sw.set_pending_spend(n, anchor, activity_id);
            }
            sw.arm_redrive(PendingRedrive {
                activity_id,
                anchor,
                nullifiers,
                st_bytes,
                attempts,
            });
        }
        Ok(())
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
        let conn = rusqlite::Connection::open(path)
            .map_err(|e| FileShieldedStoreError(format!("open sqlite: {e}")))?;
        // Pragmas must be applied before the schema is touched. They survive
        // for the lifetime of the connection; WAL also persists for any
        // subsequent reopen on the same file until explicitly changed.
        for (k, v) in [
            ("journal_mode", "WAL"),
            ("synchronous", "NORMAL"),
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
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
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
        let conn = self.pending_conn.lock().expect("pending_conn mutex");
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
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
            let nullifier_blob: Vec<u8> = redrive.nullifiers.iter().flatten().copied().collect();
            conn.execute(
                "INSERT OR REPLACE INTO shielded_pending_spends \
                 (wallet_id, account_index, activity_id, anchor, nullifiers, st_bytes, attempts) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    id.wallet_id.as_slice(),
                    id.account_index,
                    redrive.activity_id.as_slice(),
                    redrive.anchor.as_slice(),
                    nullifier_blob,
                    redrive.st_bytes,
                    redrive.attempts,
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
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
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
        // The redrive table IS durable (unlike the rest of subwallet
        // state), so purging the in-memory map alone would leave this
        // wallet's rows to rehydrate stale reservations / rebroadcast
        // state on the next open. Delete them first, scoped by
        // wallet_id — SQL before memory, so an Err return means neither
        // store was touched (fail-atomic) rather than a memory purge
        // the caller can't distinguish from a no-op.
        {
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
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
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
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
            let conn = self.pending_conn.lock().expect("pending_conn mutex");
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

    /// Guards against the "Shielded Merkle witness unavailable"
    /// spend failure on a multi-wallet shared tree.
    ///
    /// Invariant: the shared commitment tree marks EVERY position
    /// (`append_commitment(.., true)`); per-wallet ownership is
    /// tracked separately in the notes store. Appending a commitment
    /// as `Ephemeral` unless the owning wallet's IVK recognizes it in
    /// that very sync pass is wrong: with multiple wallets sharing
    /// one tree and binding at different times, a note appended
    /// before its owner binds stays Ephemeral forever — shardtree has
    /// no retroactive marking — so the balance shows but the spend
    /// fails to build a witness (on disk: every position
    /// un-witnessable, missing internal nodes at `Level(2) index 0` /
    /// `Level(1) index 2`).
    ///
    /// This test asserts
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
}
