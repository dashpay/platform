//! [`SqlitePersister`] — the canonical `PlatformWalletPersistence` impl.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};

use platform_wallet::changeset::{
    ClientStartState, Merge, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::backup::{self, BackupKind};
use crate::sqlite::buffer::Buffer;
use crate::sqlite::config::{FlushMode, SqlitePersisterConfig, Synchronous};
use crate::sqlite::error::{AutoBackupOperation, WalletStorageError};
use crate::sqlite::reports::{CommitReport, DeleteWalletReport};
use crate::sqlite::schema;
use crate::sqlite::util::permissions::{apply_secure_permissions, precreate_secure};
use crate::sqlite::util::safe_cast;

/// Sub-areas of `ClientStartState` that `load()` does not yet
/// reconstruct (blocked on upstream `Wallet::from_persisted`).
///
/// Surfaced via the structured `tracing::info!` summary on every
/// `load()` (`unimplemented` + `wallets_pending_rehydration` fields).
pub(crate) const LOAD_UNIMPLEMENTED: &[&str] = &["ClientStartState::wallets"];

/// Outcome of a `prune_backups` call.
///
/// Invariant: `kept == total_eligible - removed.len()`. A file is
/// counted as `kept` if it survived the policy (retained-by-rule) OR
/// if `remove_file` failed (`failed_removals` is a subset of `kept`).
/// Either way, the file is still on disk after this call.
#[derive(Debug)]
pub struct PruneReport {
    /// Paths that were unlinked, sorted oldest-first by filename
    /// timestamp.
    pub removed: Vec<PathBuf>,
    /// Files still on disk after this call. Equals
    /// `total_eligible - removed.len()` and includes every
    /// `failed_removals` entry — a file that couldn't be unlinked is
    /// still on disk and therefore "kept".
    pub kept: usize,
    /// Files we tried to remove but couldn't, paired with the
    /// underlying `io::Error`. Returned as part of `Ok(report)` so a
    /// partial failure surfaces every removed AND every failed entry
    /// — the caller can re-invoke `prune_backups` to retry just the
    /// stragglers.
    pub failed_removals: Vec<(PathBuf, std::io::Error)>,
}

/// Retention policy for `prune_backups`.
///
/// **AND-semantics**: a file is kept iff it satisfies BOTH rules. A
/// policy with `keep_last_n = Some(3)` and `max_age = Some(30d)` keeps
/// at most the three newest backups AND only those younger than 30
/// days — a four-day-old backup that's the fifth-newest is removed.
/// `RetentionPolicy::default()` (both `None`) keeps every file.
#[derive(Debug, Clone, Copy, Default)]
pub struct RetentionPolicy {
    pub keep_last_n: Option<usize>,
    pub max_age: Option<std::time::Duration>,
}

impl RetentionPolicy {
    pub fn keep_last(n: usize) -> Self {
        Self {
            keep_last_n: Some(n),
            max_age: None,
        }
    }
    pub fn older_than(d: std::time::Duration) -> Self {
        Self {
            keep_last_n: None,
            max_age: Some(d),
        }
    }
}

/// SQLite-backed `PlatformWalletPersistence`.
pub struct SqlitePersister {
    config: SqlitePersisterConfig,
    // Single connection serializes reads through the write lock.
    // Acceptable for the current workload (per-wallet operations, small
    // read footprint); a read-only pool over the same WAL-mode file is
    // the planned follow-up if read contention becomes measurable.
    conn: Arc<Mutex<Connection>>,
    buffer: Buffer,
    /// Test-only one-shot injector for `flush_inner`. Lives on the
    /// struct so `force_next_flush_to_fail` can survive across `&self`
    /// calls. Production builds keep the slot but never write to it
    /// (no public setter outside `#[cfg(any(test, feature = "__test-helpers"))]`).
    #[cfg(any(test, feature = "__test-helpers"))]
    primed_flush_error: Mutex<Option<WalletStorageError>>,
    /// Test-only one-shot injection consumed by `delete_wallet`'s
    /// pre-flush phase. Lets a test assert the buffer-restore and
    /// skip-backup semantics without provoking a real SQL error.
    #[cfg(any(test, feature = "__test-helpers"))]
    primed_pre_flush_error: Mutex<Option<WalletStorageError>>,
}

impl SqlitePersister {
    /// Open or create the SQLite DB at `config.path`. Applies pragmas,
    /// asserts integrity on a pre-existing DB, runs migrations,
    /// optionally takes a pre-migration auto-backup.
    ///
    /// # Errors
    ///
    /// - [`WalletStorageError::ConfigInvalid`] — rejected
    ///   [`SqlitePersisterConfig`] field (e.g. `synchronous = Off`).
    /// - [`WalletStorageError::Io`] (kind `NotFound`) — the parent of
    ///   `config.path` does not exist. The persister refuses to create
    ///   parent directories silently.
    /// - [`WalletStorageError::ForeignKeysNotEnforced`] — the linked
    ///   SQLite build silently ignores `PRAGMA foreign_keys = ON`
    ///   (no FK support compiled in).
    /// - [`WalletStorageError::SchemaVersionUnsupported`] — the DB
    ///   carries a `refinery_schema_history` row beyond what this
    ///   binary can apply. Symmetric with `restore_from`'s gate.
    /// - [`WalletStorageError::IntegrityCheckFailed`] —
    ///   `PRAGMA integrity_check` on the pre-existing DB returned a
    ///   non-`ok` report. Raised BEFORE migrations alter the file so
    ///   corruption is never silently migrated.
    /// - [`WalletStorageError::Migration`] — refinery failed mid-run.
    /// - [`WalletStorageError::AutoBackupDirUnwritable`] /
    ///   [`WalletStorageError::AutoBackupDisabled`] — the
    ///   pre-migration auto-backup couldn't materialise.
    pub fn open(config: SqlitePersisterConfig) -> Result<Self, WalletStorageError> {
        validate_config(&config)?;
        if let Some(parent) = config.path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                // Parent dir must exist — refuse to create it silently so
                // "bad path" stays a typed error.
                return Err(WalletStorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("database parent directory not found: {}", parent.display()),
                )));
            }
        }

        // Pre-create the DB file owner-only (0600) with O_EXCL BEFORE
        // rusqlite opens it: the file is born at 0600 (no umask window)
        // and an attacker-planted symlink at the path makes the create
        // fail rather than redirect (no chmod-by-path TOCTOU). A no-op
        // when the DB already exists. Brings the SQLite path to parity
        // with the secrets-vault file path.
        precreate_secure(&config.path)?;

        // Open the connection AND apply pragmas before checking for
        // pending migrations so the integrity probe sees the configured
        // journal mode and busy timeout. `open_conn` enables foreign-key
        // enforcement and asserts the read-back before any write lands.
        let mut conn =
            crate::sqlite::conn::open_conn(&config.path, crate::sqlite::conn::Access::ReadWrite)?;
        // Re-tighten to 0600 on Unix (idempotent on re-open) and sweep
        // the WAL/SHM sidecars that SQLite creates after open.
        apply_secure_permissions(&config.path)?;
        apply_pragmas(&mut conn, &config)?;

        // Determine whether `schema_history` exists *before* we run
        // migrations — that's the signal for "is this DB pre-existing or
        // brand-new?". Errors from the underlying query are propagated,
        // not silently treated as "no history".
        let had_schema_history = crate::sqlite::migrations::has_schema_history(&conn)?;
        // Run integrity_check on a pre-existing DB BEFORE migrations alter
        // it. Bit-rot or escaped-WAL corruption detected here surfaces as
        // the typed `IntegrityCheckFailed` before any schema mutation
        // lands. The pre-migration auto-backup snapshots the live state,
        // so without this gate a corrupt DB gets backed up and migrated in
        // the same pass — making the auto-backup useless for rollback.
        if had_schema_history {
            crate::sqlite::backup::run_integrity_check(&conn, |report| {
                WalletStorageError::IntegrityCheckFailed { report }
            })?;
        }
        // Refuse to open a DB produced by a newer binary — refinery's
        // run() would no-op on pending_count==0, after which blob decoders
        // would see forward-schema bytes. Symmetric with restore_from's
        // max-version gate (both call the same helper).
        if had_schema_history {
            crate::sqlite::migrations::assert_schema_version_supported(&conn)?;
        }
        let pending = crate::sqlite::migrations::embedded_migrations();
        let pending_count = if had_schema_history {
            count_pending(&mut conn, &pending)?
        } else {
            pending.len()
        };

        if pending_count > 0 && had_schema_history {
            let from = current_schema_version(&conn)?.unwrap_or(0);
            let to = pending.iter().map(|(v, _)| *v).max().unwrap_or(from);
            run_auto_backup(
                &conn,
                config.auto_backup_dir.as_deref(),
                BackupKind::PreMigration { from, to },
                AutoBackupOperation::OpenMigration,
            )?;
        }

        // Apply migrations through the typed-error chokepoint.
        let _report = crate::sqlite::migrations::run_for_open(&mut conn)?;

        Ok(Self {
            config,
            conn: Arc::new(Mutex::new(conn)),
            buffer: Buffer::new(),
            #[cfg(any(test, feature = "__test-helpers"))]
            primed_flush_error: Mutex::new(None),
            #[cfg(any(test, feature = "__test-helpers"))]
            primed_pre_flush_error: Mutex::new(None),
        })
    }

    /// Take a manual online backup. `dest` may be a directory (auto-
    /// named `wallet-<ts>.db`) or a full file path (must not pre-exist).
    pub fn backup_to(&self, dest: &Path) -> Result<PathBuf, WalletStorageError> {
        let resolved = if dest.is_dir() {
            dest.join(backup::manual_backup_filename())
        } else {
            if dest.exists() {
                return Err(WalletStorageError::BackupDestinationExists {
                    path: dest.to_path_buf(),
                });
            }
            dest.to_path_buf()
        };
        let conn = self.conn()?;
        backup::run_to(&conn, &resolved)?;
        Ok(resolved.canonicalize().unwrap_or(resolved))
    }

    /// Restore a backup over `dest_db_path`. Destination must not be
    /// open in this process. Associated function — no `&self`.
    ///
    /// Takes a pre-restore auto-backup of the live destination
    /// database (when `auto_backup_dir` is `Some`) before persisting
    /// the staged source. Refuses with
    /// [`WalletStorageError::AutoBackupDisabled`] when the directory
    /// is `None`; pass `auto_backup_dir = None` only via the CLI's
    /// `--no-auto-backup` flag (or directly through
    /// [`restore_from_skip_backup`](Self::restore_from_skip_backup)).
    ///
    /// # Cross-process rollback caveat
    ///
    /// The pre-restore auto-backup is taken BEFORE the SQLite-native
    /// `BEGIN EXCLUSIVE` that guards the restore body. Under concurrent
    /// cross-process access the rollback point may therefore miss writes
    /// a peer committed between the snapshot and the lock. Serializing
    /// restore intent across processes is the caller's responsibility.
    pub fn restore_from(
        dest_db_path: &Path,
        src_backup: &Path,
        auto_backup_dir: Option<&Path>,
    ) -> Result<(), WalletStorageError> {
        Self::restore_from_inner(dest_db_path, src_backup, auto_backup_dir, false)
    }

    /// Restore a backup over `dest_db_path` WITHOUT taking a
    /// pre-restore auto-backup.
    ///
    /// Library consumers should prefer [`restore_from`](Self::restore_from)
    /// — it's safe by default. This entry point exists so the CLI's
    /// `--no-auto-backup` flag can deliver on its name regardless of
    /// `auto_backup_dir`.
    pub fn restore_from_skip_backup(
        dest_db_path: &Path,
        src_backup: &Path,
    ) -> Result<(), WalletStorageError> {
        Self::restore_from_inner(dest_db_path, src_backup, None, true)
    }

    fn restore_from_inner(
        dest_db_path: &Path,
        src_backup: &Path,
        auto_backup_dir: Option<&Path>,
        skip_backup: bool,
    ) -> Result<(), WalletStorageError> {
        if !skip_backup && dest_db_path.exists() {
            let dir = auto_backup_dir.ok_or(WalletStorageError::AutoBackupDisabled {
                operation: AutoBackupOperation::Restore,
            })?;
            // Open the destination read-only just long enough to
            // page-stream a snapshot to disk under auto_backup_dir.
            let dest_conn = crate::sqlite::conn::open_conn(
                dest_db_path,
                crate::sqlite::conn::Access::ReadOnly,
            )?;
            run_auto_backup(
                &dest_conn,
                Some(dir),
                BackupKind::PreRestore,
                AutoBackupOperation::Restore,
            )?;
            drop(dest_conn);
        }
        // No row-count fingerprint guards the snapshot → EXCLUSIVE
        // window: `backup::restore_from` holds a SQLite-native `BEGIN
        // EXCLUSIVE` over the whole restore body, so peers that race the
        // snapshot are excluded from there on. A count fingerprint would
        // miss in-place UPDATEs on single-row tables and give operators
        // false confidence; callers needing a quiesced rollback point
        // must serialize restore intent at the application layer.
        backup::restore_from(dest_db_path, src_backup)
    }

    /// Apply retention to a directory of `wallet-*.db` (and/or
    /// `pre-*-*.db`) files.
    pub fn prune_backups(
        &self,
        dir: &Path,
        policy: RetentionPolicy,
    ) -> Result<PruneReport, WalletStorageError> {
        backup::prune(dir, policy)
    }

    /// Cascade-delete every row owned by `wallet_id`. Takes a
    /// pre-delete auto-backup before the cascade and refuses if
    /// `auto_backup_dir` is `None`. The library-API, safe-by-default
    /// route.
    ///
    /// To skip the auto-backup explicitly — wired up by the CLI's
    /// `--no-auto-backup` — call
    /// [`delete_wallet_skip_backup`](Self::delete_wallet_skip_backup).
    ///
    /// # Cross-process rollback caveat
    ///
    /// The pre-delete auto-backup is taken BEFORE the SQLite-native
    /// `BEGIN EXCLUSIVE` that guards the cascade. Under concurrent
    /// cross-process access the rollback point may therefore miss writes
    /// a peer committed between the snapshot and the lock. Serializing
    /// delete intent across processes is the caller's responsibility.
    ///
    /// # Racing stores
    ///
    /// Calls to `store(wallet_id, ...)` for the same wallet while
    /// `delete_wallet` is in progress will be **discarded** after the
    /// delete commits. The store call may return `Ok(())` (in
    /// `FlushMode::Manual` it lands in the buffer), but its data does
    /// not survive the delete — the post-commit re-drain inside
    /// `delete_wallet` removes any buffered changeset that arrived
    /// during the delete window. Synchronize at the caller layer if
    /// you need different semantics.
    pub fn delete_wallet(
        &self,
        wallet_id: WalletId,
    ) -> Result<DeleteWalletReport, WalletStorageError> {
        self.delete_wallet_inner(wallet_id, false)
    }

    /// Cascade-delete every row owned by `wallet_id` WITHOUT taking
    /// an auto-backup.
    ///
    /// Library consumers should prefer [`delete_wallet`](Self::delete_wallet)
    /// — it's safe by default. This entry point exists so the CLI's
    /// `--no-auto-backup` flag can deliver on its name regardless of
    /// `auto_backup_dir`. Returns `DeleteWalletReport.backup_path =
    /// None` to signal the backup was intentionally skipped.
    pub fn delete_wallet_skip_backup(
        &self,
        wallet_id: WalletId,
    ) -> Result<DeleteWalletReport, WalletStorageError> {
        self.delete_wallet_inner(wallet_id, true)
    }

    fn delete_wallet_inner(
        &self,
        wallet_id: WalletId,
        skip_backup: bool,
    ) -> Result<DeleteWalletReport, WalletStorageError> {
        // Acquire the connection mutex FIRST so concurrent in-process
        // `store()` calls block on it. Cross-process peers (other
        // rusqlite Connections / sibling `SqlitePersister`s) are excluded
        // by `BEGIN EXCLUSIVE` below — the in-process mutex alone never
        // gave that guarantee.
        let mut conn = self.conn()?;

        // Drain the buffered changeset so a later flush can't
        // resurrect the wallet, and so the wallet counts as existing
        // even when its only state is buffered. Hold the drained value
        // in `drained_slot` and only consume it AFTER tx.commit().
        let drained = self.buffer.take_for_flush(&wallet_id)?;
        let had_buffered = drained.is_some();
        let drained_slot: std::cell::Cell<Option<PlatformWalletChangeSet>> =
            std::cell::Cell::new(drained);

        // Helper: any pre-commit failure must restore the changeset so
        // we don't lose pending writes on a delete that didn't happen.
        let restore_buffer = |slot: &std::cell::Cell<Option<PlatformWalletChangeSet>>| {
            if let Some(cs) = slot.take() {
                if let Err(e) = self.buffer.restore(wallet_id, cs) {
                    tracing::error!(
                        wallet_id = %hex::encode(wallet_id),
                        error_kind = e.error_kind_str(),
                        "buffer restore failed during delete_wallet error path — changeset lost"
                    );
                }
            }
        };

        let result: Result<DeleteWalletReport, WalletStorageError> = (|| {
            // Pre-flight existence check on the bare conn (no tx) so
            // we don't waste a backup file on an unknown wallet.
            let exists_pre_flush = conn
                .query_row(
                    "SELECT 1 FROM wallet_metadata WHERE wallet_id = ?1",
                    rusqlite::params![wallet_id.as_slice()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !had_buffered && !exists_pre_flush {
                return Err(WalletStorageError::WalletNotFound { wallet_id });
            }

            // Test-only injector — force the pre-flush below to fail with
            // the primed error without depending on a real SQL failure.
            // Keeps the test free of FK-poisoning scaffolding.
            #[cfg(any(test, feature = "__test-helpers"))]
            let primed_pre_flush_error = self.consume_primed_pre_flush_error();

            // Flush the drained buffer to disk BEFORE `run_auto_backup`
            // so the pre-delete snapshot includes every pending write.
            // Without this the backup captures only already-persisted
            // state and rollback-from-backup cannot recover the buffered
            // (lost) data.
            //
            // The flush opens its own EXCLUSIVE tx and commits;
            // `run_auto_backup` then runs against the freshly-flushed
            // DB. On flush failure we restore the buffer via the outer
            // `restore_buffer` helper and abort the delete.
            //
            // The cascade-side backup runs BEFORE the cascade's
            // `BEGIN EXCLUSIVE` because rusqlite's `Backup::new` can't
            // establish a backup whose source connection holds an
            // active write tx on its own DB — `sqlite3_backup_step`
            // would deadlock against the in-flight EXCLUSIVE.
            if let Some(cs) = drained_slot.take() {
                #[cfg(any(test, feature = "__test-helpers"))]
                if let Some(primed) = primed_pre_flush_error {
                    drained_slot.set(Some(cs));
                    return Err(primed);
                }
                let pre_flush_tx =
                    conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;
                if let Err(e) = apply_changeset_to_tx(&pre_flush_tx, &wallet_id, &cs) {
                    let _ = pre_flush_tx.rollback();
                    drained_slot.set(Some(cs));
                    return Err(e);
                }
                if let Err(e) = pre_flush_tx.commit() {
                    drained_slot.set(Some(cs));
                    return Err(WalletStorageError::Sqlite(e));
                }
            }

            // Concurrent-peer detection relies on the auto-backup taken
            // before the cascade plus the SQLite-native `BEGIN EXCLUSIVE`
            // below — not a row-count fingerprint, which an in-place
            // UPDATE on a single-row table would evade. Pre-flushing
            // before the backup ensures the snapshot captures every
            // buffered write.
            let backup_path = if skip_backup {
                None
            } else {
                run_auto_backup(
                    &conn,
                    self.config.auto_backup_dir.as_deref(),
                    BackupKind::PreDelete { wallet_id },
                    AutoBackupOperation::DeleteWallet,
                )?
            };

            // SQLite-native EXCLUSIVE for the cascade window. Excludes
            // cross-process peers (other rusqlite Connections, sibling
            // `SqlitePersister`s) that would otherwise commit rows for
            // `wallet_id` during the cascade. The in-process mutex on
            // `conn` alone never gave that guarantee. Peers waiting on
            // the lock back off via SQLite's `busy_timeout`.
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;

            // Deleting the parent `wallet_metadata` row drives the whole
            // cleanup: native `ON DELETE CASCADE` removes every FK-bearing
            // per-wallet/per-identity table, and the AFTER DELETE triggers
            // broom every wallet/identity-scoped `meta_*` row (parentless
            // included). No per-table accounting is needed — the
            // cascade-completeness test asserts no row survives.
            crate::sqlite::schema::wallet_meta::delete(&tx, &wallet_id)?;
            tx.commit()?;
            // Commit succeeded — drop the original drained changeset.
            drop(drained_slot.take());
            // Re-drain any changeset a Manual-mode store dropped into the
            // buffer while we held conn. The wallet is gone — these
            // writes are intentionally void.
            if let Ok(Some(_late)) = self.buffer.take_for_flush(&wallet_id) {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "discarded racing buffered changeset after delete_wallet commit"
                );
            }
            Ok(DeleteWalletReport {
                wallet_id,
                backup_path,
            })
        })();

        if result.is_err() {
            restore_buffer(&drained_slot);
        }
        result
    }

    /// Attempt to flush every dirty wallet, regardless of flush mode.
    ///
    /// In `Manual` mode this is the only way pending writes become
    /// durable. In `Immediate` mode the buffer is normally empty (each
    /// `store` flushes inline) but a transient failure during `store`
    /// leaves the changeset in the buffer — `commit_writes` is the
    /// retry path that drains those leftovers.
    ///
    /// Continues past per-wallet failures instead of fails-fast.
    /// Each wallet's flush outcome lands on the returned
    /// [`CommitReport`]: `succeeded` for durable writes, `failed` for
    /// the classified `PersistenceError`. `still_pending` only fills
    /// when a `LockPoisoned` short-circuit prevents the loop from
    /// attempting the remaining wallets.
    ///
    /// Returns `Err` ONLY when even enumerating the dirty set fails
    /// (e.g. the buffer mutex is poisoned). Once the loop starts,
    /// every dirty wallet has a slot in the report.
    pub fn commit_writes(&self) -> Result<CommitReport, PersistenceError> {
        self.commit_writes_inner()
    }

    fn commit_writes_inner(&self) -> Result<CommitReport, PersistenceError> {
        let mut report = CommitReport {
            succeeded: Vec::new(),
            failed: Vec::new(),
            still_pending: Vec::new(),
        };
        // Even in `FlushMode::Immediate` the buffer can be non-empty:
        // a transient failure during `store()` re-merges the changeset
        // back into the buffer via `handle_flush_error`. The retry path
        // — `commit_writes()` — has to drain that leftover regardless
        // of flush mode, otherwise transient-failure data sits there
        // until the next per-wallet `store` happens to retry it.
        let dirty = self
            .buffer
            .dirty_wallets()
            .map_err(PersistenceError::from)?;
        let mut iter = dirty.into_iter();
        while let Some(id) = iter.next() {
            match self.flush_inner(&id) {
                Ok(()) => report.succeeded.push(id),
                Err(PersistenceError::LockPoisoned) => {
                    // Mutex is gone — no point hammering the remaining
                    // wallets. Record this one as failed and shovel the
                    // rest into still_pending so the caller knows what
                    // was never attempted.
                    report.failed.push((id, PersistenceError::LockPoisoned));
                    report.still_pending.extend(iter);
                    return Ok(report);
                }
                Err(e) => report.failed.push((id, e)),
            }
        }
        Ok(report)
    }

    /// Lock the write connection.
    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, WalletStorageError> {
        self.conn
            .lock()
            .map_err(|_| WalletStorageError::LockPoisoned)
    }

    // The feature is named with Cargo's `__` prefix convention to
    // signal "not part of the public API; downstream MUST NOT enable
    // it" (https://doc.rust-lang.org/cargo/reference/features.html).
    // The methods themselves are `#[doc(hidden)]` so they don't show
    // up on docs.rs even when the feature is on.
    /// Test-only: borrow the write connection.
    ///
    /// Tests use this to seed `wallet_metadata` rows directly, run
    /// SELECTs against tables that aren't part of the public surface,
    /// or probe `PRAGMA foreign_keys` / `PRAGMA journal_mode`. Gated
    /// behind `cfg(test)` and the `__test-helpers` feature —
    /// downstream crates MUST NOT enable it.
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn lock_conn_for_test(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("conn mutex poisoned")
    }

    /// Test-only: read the resolved config. Same visibility rules as
    /// [`lock_conn_for_test`](Self::lock_conn_for_test).
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn config_for_test(&self) -> &SqlitePersisterConfig {
        &self.config
    }

    fn flush_inner(&self, wallet_id: &WalletId) -> Result<(), PersistenceError> {
        let cs = self
            .buffer
            .take_for_flush(wallet_id)
            .map_err(PersistenceError::from)?;
        let Some(cs) = cs else { return Ok(()) };

        // Test-only injector: surface a primed failure without ever
        // touching SQL so take/restore semantics are exercised end-to-end.
        #[cfg(any(test, feature = "__test-helpers"))]
        if let Some(injected) = self.consume_primed_flush_error() {
            return self.handle_flush_error(wallet_id, cs, injected);
        }

        match self.write_changeset_in_one_tx(wallet_id, &cs) {
            Ok(()) => Ok(()),
            Err(e) => self.handle_flush_error(wallet_id, cs, e),
        }
    }

    /// Apply every populated sub-changeset under one transaction and
    /// commit. Returned `Err` is the per-area / commit failure verbatim
    /// — classification + buffer restore happen one level up.
    fn write_changeset_in_one_tx(
        &self,
        wallet_id: &WalletId,
        cs: &PlatformWalletChangeSet,
    ) -> Result<(), WalletStorageError> {
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        apply_changeset_to_tx(&tx, wallet_id, cs)?;
        tx.commit()?;
        Ok(())
    }

    /// Classify the failure: transient errors restore the buffer and
    /// surface as `FlushRetryable`; everything else drops the
    /// changeset and returns the original variant.
    //
    // TODO(qa): the fatal branch below covers `LockPoisoned`, but no
    // end-to-end mutex-poison test exists (a panicking thread plus a join
    // is hard to reproduce deterministically). It is verified by hand via
    // `Mutex::lock` failure injection at the typed-error layer; anyone
    // touching the classification policy or this branch must reconfirm
    // by hand.
    fn handle_flush_error(
        &self,
        wallet_id: &WalletId,
        cs: PlatformWalletChangeSet,
        err: WalletStorageError,
    ) -> Result<(), PersistenceError> {
        let field_count = populated_field_count(&cs);
        let kind = err.error_kind_str();
        if err.is_transient() {
            // A failed restore (e.g. poisoned buffer mutex) means the
            // buffered changeset is gone — that is itself fatal and
            // must surface, not be masked by the transient signal.
            if let Err(restore_err) = self.buffer.restore(*wallet_id, cs) {
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error_kind = restore_err.error_kind_str(),
                    restored_field_count = field_count,
                    "buffer restore failed after transient flush error — changeset lost"
                );
                return Err(PersistenceError::from(restore_err));
            }
            // Narrow the error to its rusqlite source — only
            // `Sqlite(SqliteFailure(BUSY|LOCKED, _))` qualifies for
            // surfacing as `FlushRetryable`.
            let source = match err {
                WalletStorageError::Sqlite(rusq) => rusq,
                WalletStorageError::FlushRetryable { source, .. } => source,
                other => {
                    // Defensive: classifier said "transient" but source
                    // isn't rusqlite. Surface unwrapped — better than
                    // lying about the source type.
                    tracing::warn!(
                        wallet_id = %hex::encode(wallet_id),
                        error_kind = kind,
                        restored_field_count = field_count,
                        "transient classification with non-sqlite source — propagating raw"
                    );
                    return Err(PersistenceError::from(other));
                }
            };
            tracing::warn!(
                wallet_id = %hex::encode(wallet_id),
                error_kind = kind,
                restored_field_count = field_count,
                "flush failed transiently — buffer restored for retry"
            );
            Err(PersistenceError::from(WalletStorageError::FlushRetryable {
                wallet_id: *wallet_id,
                source,
            }))
        } else {
            tracing::error!(
                wallet_id = %hex::encode(wallet_id),
                error_kind = kind,
                dropped_field_count = field_count,
                "flush failed fatally — buffer wiped"
            );
            // `cs` dropped here.
            drop(cs);
            Err(PersistenceError::from(err))
        }
    }

    /// Test-only: arm a one-shot injection consumed by the next
    /// `flush_inner`. Higher-level than `FailingConnection`; useful
    /// when the test doesn't care which SQL error fires, only how the
    /// wrapper reacts.
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn force_next_flush_to_fail(&self, err: WalletStorageError) {
        *self.primed_flush_error.lock().expect("primed_flush_error") = Some(err);
    }

    #[cfg(any(test, feature = "__test-helpers"))]
    fn consume_primed_flush_error(&self) -> Option<WalletStorageError> {
        self.primed_flush_error
            .lock()
            .expect("primed_flush_error")
            .take()
    }

    /// Test-only: arm a one-shot pre-flush failure for the next
    /// `delete_wallet` call. The injection fires only when there is
    /// a drained buffered changeset to flush — i.e. when `delete_wallet`
    /// actually exercises the pre-flush branch.
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn force_next_pre_flush_to_fail(&self, err: WalletStorageError) {
        *self
            .primed_pre_flush_error
            .lock()
            .expect("primed_pre_flush_error") = Some(err);
    }

    #[cfg(any(test, feature = "__test-helpers"))]
    fn consume_primed_pre_flush_error(&self) -> Option<WalletStorageError> {
        self.primed_pre_flush_error
            .lock()
            .expect("primed_pre_flush_error")
            .take()
    }

    /// Test-only: probe whether the wallet has a buffered changeset.
    /// Used to assert the buffer survives a failed pre-flush without
    /// consuming it.
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn buffer_has_changeset_for_test(&self, wallet_id: &WalletId) -> bool {
        self.buffer
            .dirty_wallets()
            .map(|v| v.iter().any(|w| w == wallet_id))
            .unwrap_or(false)
    }
}

/// When a `Manual`-mode persister is dropped while dirty wallets remain,
/// log a structured `tracing::error!` so the silent-data-loss footgun
/// (the buffer dies with the persister) surfaces in operator logs.
///
/// We intentionally do NOT auto-flush from `Drop` — `flush_inner`
/// can fail and `Drop` cannot propagate errors, so a swallow there
/// would be a worse failure mode than the loud log. `Immediate`-mode
/// persisters are durable on every `store` so they never trip this.
impl Drop for SqlitePersister {
    fn drop(&mut self) {
        if self.config.flush_mode != FlushMode::Manual {
            return;
        }
        // `dirty_wallets` only fails on a poisoned buffer mutex. A
        // poisoned mutex on Drop already means the process is wedged;
        // we still try to surface the lost state where we can.
        let dirty = match self.buffer.dirty_wallets() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(
                    target: "platform_wallet_storage",
                    error_kind = e.error_kind_str(),
                    "SqlitePersister dropped with buffer mutex poisoned — uncommitted state unrecoverable"
                );
                return;
            }
        };
        if dirty.is_empty() {
            return;
        }
        // `take_for_flush` mutates the buffer (drains the changeset).
        // That is intentional here: the persister is being dropped, no
        // future caller can observe the buffer, and `populated_field_count`
        // needs to inspect the changeset to produce the diagnostic. Do
        // NOT treat `impl Drop` as side-effect-free.
        let total_fields: usize = dirty
            .iter()
            .filter_map(|id| {
                self.buffer
                    .take_for_flush(id)
                    .ok()
                    .flatten()
                    .map(|cs| populated_field_count(&cs))
            })
            .sum();
        tracing::error!(
            target: "platform_wallet_storage",
            dirty_wallets = dirty.len(),
            total_fields,
            "SqlitePersister dropped with uncommitted Manual-mode writes"
        );
    }
}

impl PlatformWalletPersistence for SqlitePersister {
    fn store_commits_inline(&self) -> bool {
        self.config.flush_mode == FlushMode::Immediate
    }

    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        // Every `flush_inner` applies the complete changeset in one SQLite
        // transaction. The current schema also has lossless token balances,
        // invitations, account pools, tracked asset locks, and
        // deferred-contact-crypto queue rows.
        // Do NOT attest WALLET_RESTORE (and therefore not provider restore):
        // `load()` still reports `ClientStartState::wallets` in
        // `LOAD_UNIMPLEMENTED`. Shielded state lives in a separate store.
        PersistenceCapabilities::ATOMIC_CHANGESETS
            .union(PersistenceCapabilities::INVITATIONS)
            .union(PersistenceCapabilities::ASSET_LOCK_FUNDING_INDICES)
            .union(PersistenceCapabilities::UNSIGNED_TOKEN_STORAGE)
            .union(PersistenceCapabilities::PENDING_CONTACT_CRYPTO)
            .union(PersistenceCapabilities::DPNS_NAME_STATES)
            .union(PersistenceCapabilities::TRACKED_ASSET_LOCKS)
            .union(PersistenceCapabilities::TRACKED_MASTERNODES)
    }

    fn persist_tracked_masternodes(
        &self,
        network: dashcore::Network,
        records: &[platform_wallet::masternode::TrackedMasternode],
    ) -> Result<(), PersistenceError> {
        let mut conn = self.conn().map_err(PersistenceError::from)?;
        let tx = conn
            .transaction()
            .map_err(|e| PersistenceError::from(WalletStorageError::from(e)))?;
        schema::tracked_masternodes::replace_all(&tx, network, records)
            .map_err(PersistenceError::from)?;
        tx.commit()
            .map_err(|e| PersistenceError::from(WalletStorageError::from(e)))
    }

    fn load_tracked_masternodes(
        &self,
        network: dashcore::Network,
    ) -> Result<Vec<platform_wallet::masternode::TrackedMasternode>, PersistenceError> {
        let conn = self.conn().map_err(PersistenceError::from)?;
        schema::tracked_masternodes::load_all(&conn, network).map_err(PersistenceError::from)
    }

    /// Merge `changeset` into the per-wallet buffer.
    ///
    /// Durability matrix:
    /// - In [`FlushMode::Immediate`] the call is **durable on `Ok`** —
    ///   one SQLite transaction wraps every populated per-table apply,
    ///   so either all sub-changesets land or none do. A transient
    ///   failure restores the buffer and surfaces
    ///   [`WalletStorageError::FlushRetryable`] wrapped in
    ///   `PersistenceError::Backend`.
    /// - In [`FlushMode::Manual`] the call only merges into the
    ///   in-memory buffer. Durability requires
    ///   [`flush`](Self::flush) (per-wallet) or
    ///   [`commit_writes`](Self::commit_writes) (every dirty wallet).
    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        self.buffer
            .store(wallet_id, changeset)
            .map_err(PersistenceError::from)?;
        match self.config.flush_mode {
            FlushMode::Immediate => self.flush_inner(&wallet_id),
            FlushMode::Manual => Ok(()),
        }
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        self.flush_inner(&wallet_id)
    }

    /// Load every wallet's start-state from disk.
    ///
    /// Populates `platform_addresses` per wallet. `wallets` stays empty
    /// pending an upstream `key_wallet::Wallet::from_persisted`
    /// constructor — the count of wallets that *would* be rehydrated is
    /// surfaced as the structured field `wallets_pending_rehydration`
    /// on the `tracing::info!` summary.
    ///
    /// Fail-hard: any row that fails to decode (or carries a malformed
    /// `wallet_id`) aborts the whole load with a typed
    /// [`WalletStorageError`]. Corruption is never silently skipped.
    ///
    /// **Query budget.** Constant w.r.t. wallet count: one `SELECT` over
    /// `wallet_metadata` for the wallet-id list plus a fixed set of
    /// grouped scans (sync state, addresses, platform-payment
    /// registrations), not a per-wallet fan-out.
    ///
    /// # Concurrency
    ///
    /// Holds the connection mutex for the duration of the read.
    /// Concurrent `store` / `flush` / `delete_wallet` calls block
    /// until `load` returns. Intended for one-shot use at process
    /// startup, not interleaved with the hot write path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::sync::Arc;
    /// use platform_wallet::changeset::PlatformWalletPersistence;
    /// use platform_wallet_storage::{SqlitePersister, SqlitePersisterConfig};
    ///
    /// # fn main() -> Result<(), platform_wallet_storage::WalletStorageError> {
    /// // Per-test isolated path — no shared state, no real wallet data.
    /// let dir = std::env::temp_dir().join(format!(
    ///     "platform-wallet-storage-doctest-{}-{}",
    ///     std::process::id(),
    ///     std::time::SystemTime::now()
    ///         .duration_since(std::time::UNIX_EPOCH)
    ///         .unwrap()
    ///         .as_nanos()
    /// ));
    /// std::fs::create_dir_all(&dir).unwrap();
    /// let db_path = dir.join("wallets.db");
    ///
    /// let config = SqlitePersisterConfig::new(&db_path);
    /// let persister: Arc<dyn PlatformWalletPersistence> =
    ///     Arc::new(SqlitePersister::open(config)?);
    ///
    /// // Empty database → empty start-state, no error.
    /// let state = persister.load().expect("load");
    /// assert!(state.platform_addresses.is_empty());
    /// assert!(state.wallets.is_empty());
    ///
    /// // Cleanup — the doctest owns the directory.
    /// drop(persister);
    /// let _ = std::fs::remove_dir_all(&dir);
    /// # Ok(())
    /// # }
    /// ```
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // TODO: repopulate ClientStartState.wallets. The
        // identity/contacts/asset-lock readers exist, but the
        // `Wallet::from_persisted` wiring lands with the rehydration work;
        // until then `load()` only rebuilds `platform_addresses` and emits
        // a structured-log summary so operators see the gap.
        let conn = self.conn().map_err(PersistenceError::from)?;
        let mut state = ClientStartState::default();

        let addrs_all = schema::platform_addrs::load_all(&conn).map_err(PersistenceError::from)?;
        let wallets_seen = addrs_all.len();
        let mut addresses_loaded: usize = 0;

        for (wallet_id, (addrs, count)) in addrs_all {
            // Omit a wallet that carries no platform state at all: no
            // per-account registrations, no addresses, and all sync
            // watermarks zero. Such a wallet contributes nothing to a
            // restored provider.
            if count > 0
                || !addrs.per_account.is_empty()
                || addrs.sync_height > 0
                || addrs.sync_timestamp > 0
                || addrs.last_known_recent_block > 0
            {
                addresses_loaded += count;
                state.platform_addresses.insert(wallet_id, addrs);
            }
        }

        tracing::info!(
            wallets_seen,
            addresses_loaded,
            wallets_rehydrated = 0usize,
            wallets_pending_rehydration = wallets_seen,
            unimplemented = ?LOAD_UNIMPLEMENTED,
            "load() summary"
        );
        Ok(state)
    }

    fn get_core_tx_record(
        &self,
        wallet_id: WalletId,
        txid: &dashcore::Txid,
    ) -> Result<
        Option<key_wallet::managed_account::transaction_record::TransactionRecord>,
        PersistenceError,
    > {
        let conn = self.conn().map_err(PersistenceError::from)?;
        schema::core_state::get_tx_record(&conn, &wallet_id, txid).map_err(PersistenceError::from)
    }
}

// ----- Helpers -----

/// Count of top-level slots that carry any data. Feeds the persister's
/// `restored_field_count` / `dropped_field_count` tracing fields so
/// operators can see how much was kept or dropped on a flush retry /
/// fatal failure. Computed here from the public `PlatformWalletChangeSet`
/// fields + `Merge::is_empty()` so no storage-only helper leaks into
/// the `rs-platform-wallet` public API.
fn populated_field_count(cs: &PlatformWalletChangeSet) -> usize {
    [
        cs.core.is_empty(),
        cs.identities.is_empty(),
        cs.identity_keys.is_empty(),
        cs.contacts.is_empty(),
        cs.platform_addresses.is_empty(),
        cs.asset_locks.is_empty(),
        cs.token_balances.is_empty(),
        cs.dashpay_profiles.as_ref().is_none_or(|m| m.is_empty()),
        cs.dashpay_payments_overlay
            .as_ref()
            .is_none_or(|m| m.is_empty()),
        cs.wallet_metadata.is_none(),
        cs.account_registrations.is_empty(),
        cs.account_address_pools.is_empty(),
    ]
    .iter()
    .filter(|empty| !**empty)
    .count()
}

fn validate_config(config: &SqlitePersisterConfig) -> Result<(), WalletStorageError> {
    if config.synchronous == Synchronous::Off {
        return Err(WalletStorageError::ConfigInvalid {
            reason: "synchronous=Off is rejected (data-loss footgun)",
        });
    }
    // `journal_mode=Memory` keeps the rollback journal in RAM and
    // `journal_mode=Off` disables it outright. Either turns crash-
    // safety into a coin flip for a wallet DB — reject loudly instead
    // of silently corrupting on the next power loss.
    match config.journal_mode {
        crate::sqlite::config::JournalMode::Memory => {
            return Err(WalletStorageError::ConfigInvalid {
                reason: "journal_mode=Memory is rejected (crash-unsafe)",
            });
        }
        crate::sqlite::config::JournalMode::Off => {
            return Err(WalletStorageError::ConfigInvalid {
                reason: "journal_mode=Off is rejected (crash-unsafe)",
            });
        }
        _ => {}
    }
    // `busy_timeout=0` makes contended writers fail-fast with BUSY
    // instead of waiting — non-fatal, but the operator almost certainly
    // didn't mean it. Warn rather than reject because a few tests
    // legitimately want the fail-fast behaviour.
    if config.busy_timeout.is_zero() {
        tracing::warn!(
            "SqlitePersisterConfig.busy_timeout=0; contended writers will return BUSY \
             instead of waiting — set a non-zero timeout (default 5s) unless this is intentional"
        );
    }
    Ok(())
}

fn apply_pragmas(
    conn: &mut Connection,
    config: &SqlitePersisterConfig,
) -> Result<(), WalletStorageError> {
    // `foreign_keys` is enabled + read-back-asserted in
    // `crate::sqlite::conn::open_conn`, the single open choke-point.
    conn.pragma_update(None, "journal_mode", config.journal_mode.pragma_value())?;
    conn.pragma_update(None, "synchronous", config.synchronous.pragma_value())?;
    let ms = safe_cast::u64_to_i64(
        "busy_timeout_ms",
        u64::try_from(config.busy_timeout.as_millis()).unwrap_or(i64::MAX as u64),
    )?;
    conn.pragma_update(None, "busy_timeout", ms)?;
    Ok(())
}

/// Apply every populated sub-changeset of `cs` against the supplied
/// SQLite transaction. Does not commit; the caller owns the tx
/// lifecycle. Splitting this out from `write_changeset_in_one_tx`
/// lets `delete_wallet_inner` flush a drained buffer into a bespoke
/// pre-delete tx without re-opening the connection.
fn apply_changeset_to_tx(
    tx: &rusqlite::Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformWalletChangeSet,
) -> Result<(), WalletStorageError> {
    if let Some(meta) = cs.wallet_metadata.as_ref() {
        schema::wallet_meta::upsert(tx, wallet_id, meta)?;
    }
    if !cs.account_registrations.is_empty() {
        schema::accounts::apply_registrations(tx, wallet_id, &cs.account_registrations)?;
    }
    if !cs.account_address_pools.is_empty() {
        schema::accounts::apply_pools(tx, wallet_id, &cs.account_address_pools)?;
    }
    if !cs.pending_contact_crypto_added.is_empty() || !cs.pending_contact_crypto_cleared.is_empty()
    {
        schema::pending_contact_crypto::apply_pending_contact_crypto(
            tx,
            wallet_id,
            &cs.pending_contact_crypto_added,
            &cs.pending_contact_crypto_cleared,
        )?;
    }
    if let Some(core) = cs.core.as_ref() {
        schema::core_state::apply(tx, wallet_id, core)?;
    }
    if let Some(identities) = cs.identities.as_ref() {
        schema::identities::apply(tx, wallet_id, identities)?;
    }
    if let Some(keys) = cs.identity_keys.as_ref() {
        schema::identity_keys::apply(tx, wallet_id, keys)?;
    }
    if let Some(contacts) = cs.contacts.as_ref() {
        schema::contacts::apply(tx, wallet_id, contacts)?;
    }
    if let Some(addrs) = cs.platform_addresses.as_ref() {
        schema::platform_addrs::apply(tx, wallet_id, addrs)?;
    }
    if let Some(locks) = cs.asset_locks.as_ref() {
        schema::asset_locks::apply(tx, wallet_id, locks)?;
    }
    if let Some(invitations) = cs.invitations.as_ref() {
        schema::invitations::apply(tx, wallet_id, invitations)?;
    }
    if let Some(dpns_name_states) = cs.dpns_name_states.as_ref() {
        schema::dpns_name_states::apply(tx, wallet_id, dpns_name_states)?;
    }
    if let Some(balances) = cs.token_balances.as_ref() {
        schema::token_balances::apply(tx, wallet_id, balances)?;
    }
    if cs.dashpay_profiles.is_some() || cs.dashpay_payments_overlay.is_some() {
        schema::dashpay::apply(
            tx,
            wallet_id,
            cs.dashpay_profiles.as_ref(),
            cs.dashpay_payments_overlay.as_ref(),
        )?;
    }
    Ok(())
}

/// Take a single auto-backup. Shared code path for open-time
/// (pre-migration), pre-restore, and pre-delete invocations. Returns
/// the absolute path written, or [`WalletStorageError::AutoBackupDisabled`]
/// when `auto_backup_dir` is `None`.
pub(crate) fn run_auto_backup(
    src_conn: &Connection,
    auto_backup_dir: Option<&Path>,
    kind: BackupKind,
    operation: AutoBackupOperation,
) -> Result<Option<PathBuf>, WalletStorageError> {
    let Some(dir) = auto_backup_dir else {
        return Err(WalletStorageError::AutoBackupDisabled { operation });
    };
    ensure_dir(dir)?;
    let dest = dir.join(backup::auto_backup_filename(kind));
    backup::run_to(src_conn, &dest)?;
    Ok(Some(dest))
}

fn ensure_dir(dir: &Path) -> Result<(), WalletStorageError> {
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|source| {
            WalletStorageError::AutoBackupDirUnwritable {
                dir: dir.to_path_buf(),
                source,
            }
        })?;
    }
    // Best-effort writability probe via `NamedTempFile` (unguessable
    // name, no race against concurrent persister opens). This is TOCTOU
    // by construction — the dir CAN flip to unwritable between the probe
    // and `backup::run_to` below — but the real write has its own error
    // path, so the worst case is the operator gets the typed error from
    // the actual backup attempt instead of this fast-fail probe.
    match tempfile::NamedTempFile::new_in(dir) {
        Ok(_probe) => Ok(()),
        Err(source) => Err(WalletStorageError::AutoBackupDirUnwritable {
            dir: dir.to_path_buf(),
            source,
        }),
    }
}

fn count_pending(
    conn: &mut Connection,
    embedded: &[(i32, String)],
) -> Result<usize, WalletStorageError> {
    if !crate::sqlite::migrations::has_schema_history(conn)? {
        return Ok(embedded.len());
    }
    let applied: std::collections::HashSet<i64> = {
        let mut stmt = conn.prepare("SELECT version FROM refinery_schema_history")?;
        let rows: Result<std::collections::HashSet<i64>, _> =
            stmt.query_map([], |row| row.get::<_, i64>(0))?.collect();
        rows?
    };
    Ok(embedded
        .iter()
        .filter(|(v, _)| !applied.contains(&(*v as i64)))
        .count())
}

fn current_schema_version(conn: &Connection) -> Result<Option<i32>, WalletStorageError> {
    let row = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()?
        .flatten();
    Ok(row.map(|v| v as i32))
}
