//! [`SqlitePersister`] — the canonical `PlatformWalletPersistence` impl.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};

use platform_wallet::changeset::{
    ClientStartState, Merge, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::sqlite::backup::{self, BackupKind};
use crate::sqlite::buffer::Buffer;
use crate::sqlite::config::{FlushMode, SqlitePersisterConfig, Synchronous};
use crate::sqlite::error::{AutoBackupOperation, WalletStorageError};
use crate::sqlite::schema::{self, PER_WALLET_TABLES};
use crate::sqlite::util::permissions::apply_secure_permissions;
use crate::sqlite::util::safe_cast;

/// Sub-areas of `ClientStartState` that `load()` does not yet
/// reconstruct (blocked on upstream `Wallet::from_persisted`).
///
/// Surfaced via the structured `tracing::info!` summary on every
/// `load()` (`unimplemented` + `wallets_pending_rehydration` fields).
pub(crate) const LOAD_UNIMPLEMENTED: &[&str] = &["ClientStartState::wallets"];

/// Outcome of a `prune_backups` call.
#[derive(Debug, Clone)]
pub struct PruneReport {
    /// Paths that were unlinked, sorted oldest-first by filename
    /// timestamp.
    pub removed: Vec<PathBuf>,
    /// Number of files that remain in the directory after pruning.
    pub kept: usize,
}

/// Outcome of a [`SqlitePersister::commit_writes`] call. Carries every
/// dirty wallet's per-flush outcome so a single failed wallet doesn't
/// hide the success of its siblings (or vice-versa). The caller can
/// retry `still_pending` directly; `failed` carries the classified
/// error per wallet so transient-vs-fatal decisions stay local.
#[derive(Debug)]
pub struct CommitReport {
    /// Wallets that flushed successfully (durable on disk).
    pub succeeded: Vec<WalletId>,
    /// Wallets whose flush returned an error. The
    /// `PersistenceError` carries the classification + source per D-9.
    pub failed: Vec<(WalletId, PersistenceError)>,
    /// Wallets we never attempted because an earlier per-flush call
    /// poisoned a shared resource (today: a `LockPoisoned` short-circuit
    /// — the connection mutex is gone). Empty on the happy path.
    pub still_pending: Vec<WalletId>,
}

impl CommitReport {
    /// `true` when every dirty wallet flushed cleanly.
    pub fn is_ok(&self) -> bool {
        self.failed.is_empty() && self.still_pending.is_empty()
    }
}

/// Outcome of a `delete_wallet` / `delete_wallet_skip_backup` call.
#[derive(Debug, Clone)]
pub struct DeleteWalletReport {
    pub wallet_id: WalletId,
    /// Absolute path of the pre-delete auto-backup written before the
    /// cascade. `None` ONLY when the caller went through
    /// [`SqlitePersister::delete_wallet_skip_backup`] — every
    /// `delete_wallet` success returns `Some(path)`.
    pub backup_path: Option<PathBuf>,
    pub rows_removed_per_table: BTreeMap<&'static str, usize>,
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
    // INTENTIONAL(CODE-001): single connection serializes reads through
    // the write lock. Acceptable for current workload (per-wallet
    // operations, small read footprint); revisit if read contention
    // becomes measurable. Splitting into a read-only `r2d2` pool over
    // the same WAL-mode file is the planned follow-up.
    conn: Arc<Mutex<Connection>>,
    buffer: Buffer,
    /// Test-only one-shot injector for `flush_inner`. Lives on the
    /// struct so `force_next_flush_to_fail` can survive across `&self`
    /// calls. Production builds keep the slot but never write to it
    /// (no public setter outside `#[cfg(any(test, feature = "__test-helpers"))]`).
    #[cfg(any(test, feature = "__test-helpers"))]
    primed_flush_error: Mutex<Option<WalletStorageError>>,
}

impl SqlitePersister {
    /// Open or create the SQLite DB at `config.path`. Applies pragmas,
    /// runs migrations, optionally takes a pre-migration auto-backup.
    pub fn open(config: SqlitePersisterConfig) -> Result<Self, WalletStorageError> {
        validate_config(&config)?;
        if let Some(parent) = config.path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                // Parent dir must exist — refuse silently creating it
                // to keep "bad path" errors typed (NFR-6).
                return Err(WalletStorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("database parent directory not found: {}", parent.display()),
                )));
            }
        }

        // Open the connection AND apply pragmas before checking for
        // pending migrations so the integrity probe sees the configured
        // journal mode and busy timeout. `open_conn` enables foreign-key
        // enforcement and asserts the read-back before any write lands.
        let mut conn =
            crate::sqlite::conn::open_conn(&config.path, crate::sqlite::conn::Access::ReadWrite)?;
        // SEC-011: chmod 600 on Unix so a freshly created DB doesn't
        // inherit a wider mode from the process umask. Idempotent on
        // re-open.
        apply_secure_permissions(&config.path)?;
        apply_pragmas(&mut conn, &config)?;

        // Determine whether `schema_history` exists *before* we run
        // migrations — that's the signal for "is this DB pre-existing
        // or brand-new?" (FR-15 vs FR-16). `.optional()?` distinguishes
        // a genuine "no row" answer from a real SQL error, which we
        // propagate.
        let had_schema_history = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
                [],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        // CMT-005: refuse to open a DB produced by a newer binary —
        // refinery's run() would no-op on pending_count==0, after which
        // blob decoders would see forward-schema bytes. Symmetric with
        // restore_from's max-version gate (both call the same helper).
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

        // Apply migrations.
        let _report = crate::sqlite::migrations::run(&mut conn)?;

        Ok(Self {
            config,
            conn: Arc::new(Mutex::new(conn)),
            buffer: Buffer::new(),
            #[cfg(any(test, feature = "__test-helpers"))]
            primed_flush_error: Mutex::new(None),
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
    /// `auto_backup_dir` is `None` (FR-18). For the library-API,
    /// safe-by-default route.
    ///
    /// To skip the auto-backup explicitly — wired up by the CLI's
    /// `--no-auto-backup` — call
    /// [`delete_wallet_skip_backup`](Self::delete_wallet_skip_backup).
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
        // CMT-008: acquire the connection mutex FIRST and hold it
        // across drain → existence-check → backup → delete-transaction
        // → post-commit buffer wipe. Concurrent `store()` calls in
        // Immediate mode block on this guard (their flush takes conn);
        // Manual-mode stores can still buffer, so we re-drain after
        // commit to discard any racing writes (the wallet is going
        // away — those writes are intentionally void).
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
            // A wallet exists iff it was buffered OR persisted. Refusing
            // on a truly-unknown wallet must not waste a backup file.
            let exists_in_db = conn
                .query_row(
                    "SELECT 1 FROM wallet_metadata WHERE wallet_id = ?1",
                    rusqlite::params![wallet_id.as_slice()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !had_buffered && !exists_in_db {
                return Err(WalletStorageError::WalletNotFound { wallet_id });
            }
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
            let tx = conn.transaction()?;
            let mut rows_removed_per_table = BTreeMap::new();
            for &table in PER_WALLET_TABLES {
                // SQL injection note: `table` comes from a `&'static
                // &'static str` constant compiled into the binary. There
                // is no user input on this path.
                let n: i64 = tx
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                        rusqlite::params![wallet_id.as_slice()],
                        |row| row.get(0),
                    )
                    .optional()?
                    .unwrap_or(0);
                rows_removed_per_table.insert(table, usize::try_from(n).unwrap_or(usize::MAX));
            }
            crate::sqlite::schema::wallet_meta::delete(&tx, &wallet_id)?;
            tx.commit()?;
            // Commit succeeded — drop the original drained changeset.
            drop(drained_slot.take());
            // CMT-008: re-drain any changeset a Manual-mode store
            // dropped into the buffer while we held conn. The wallet
            // is gone — these writes are intentionally void.
            if let Ok(Some(_late)) = self.buffer.take_for_flush(&wallet_id) {
                tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "discarded racing buffered changeset after delete_wallet commit"
                );
            }
            Ok(DeleteWalletReport {
                wallet_id,
                backup_path,
                rows_removed_per_table,
            })
        })();

        if result.is_err() {
            restore_buffer(&drained_slot);
        }
        result
    }

    /// In Manual mode: attempt to flush every dirty wallet. In
    /// Immediate mode: no-op (returns an empty report).
    ///
    /// Continues past per-wallet failures instead of fails-fast (N-1).
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
        let mut report = CommitReport {
            succeeded: Vec::new(),
            failed: Vec::new(),
            still_pending: Vec::new(),
        };
        if matches!(self.config.flush_mode, FlushMode::Immediate) {
            return Ok(report);
        }
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
                    report
                        .failed
                        .push((id, PersistenceError::LockPoisoned));
                    report.still_pending.extend(iter);
                    return Ok(report);
                }
                Err(e) => report.failed.push((id, e)),
            }
        }
        Ok(report)
    }

    /// `inspect` row-count summary. With `wallet_id = Some(id)`, scoped
    /// to that wallet; otherwise total counts across all wallets.
    pub fn inspect_counts(
        &self,
        wallet_id: Option<&WalletId>,
    ) -> Result<Vec<(&'static str, usize)>, WalletStorageError> {
        let conn = self.conn()?;
        let mut out = Vec::with_capacity(PER_WALLET_TABLES.len());
        for &table in PER_WALLET_TABLES {
            // `table` is a compile-time constant — no SQL injection
            // surface despite the `format!`.
            let n: i64 = match wallet_id {
                Some(id) => conn
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                        rusqlite::params![id.as_slice()],
                        |row| row.get(0),
                    )
                    .optional()?
                    .unwrap_or(0),
                None => conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .optional()?
                    .unwrap_or(0),
            };
            out.push((table, usize::try_from(n).unwrap_or(usize::MAX)));
        }
        Ok(out)
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
        if let Some(meta) = cs.wallet_metadata.as_ref() {
            schema::wallet_meta::upsert(&tx, wallet_id, meta)?;
        }
        if !cs.account_registrations.is_empty() {
            schema::accounts::apply_registrations(&tx, wallet_id, &cs.account_registrations)?;
        }
        if !cs.account_address_pools.is_empty() {
            schema::accounts::apply_pools(&tx, wallet_id, &cs.account_address_pools)?;
        }
        if let Some(core) = cs.core.as_ref() {
            schema::core_state::apply(&tx, wallet_id, core)?;
        }
        if let Some(identities) = cs.identities.as_ref() {
            schema::identities::apply(&tx, wallet_id, identities)?;
        }
        if let Some(keys) = cs.identity_keys.as_ref() {
            schema::identity_keys::apply(&tx, wallet_id, keys)?;
        }
        if let Some(contacts) = cs.contacts.as_ref() {
            schema::contacts::apply(&tx, wallet_id, contacts)?;
        }
        if let Some(addrs) = cs.platform_addresses.as_ref() {
            schema::platform_addrs::apply(&tx, wallet_id, addrs)?;
        }
        if let Some(locks) = cs.asset_locks.as_ref() {
            schema::asset_locks::apply(&tx, wallet_id, locks)?;
        }
        if let Some(balances) = cs.token_balances.as_ref() {
            schema::token_balances::apply(&tx, wallet_id, balances)?;
        }
        if cs.dashpay_profiles.is_some() || cs.dashpay_payments_overlay.is_some() {
            schema::dashpay::apply(
                &tx,
                wallet_id,
                cs.dashpay_profiles.as_ref(),
                cs.dashpay_payments_overlay.as_ref(),
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Classify the failure: transient errors restore the buffer and
    /// surface as `FlushRetryable`; everything else drops the
    /// changeset and returns the original variant.
    //
    // TODO(qa): TC-P2-008 — the fatal branch below covers
    // `LockPoisoned`, but no end-to-end mutex-poison test exists. The
    // spec deferred it as race-prone (a panicking thread plus a join
    // is hard to reproduce deterministically); manually verified via
    // `Mutex::lock` failure injection at the typed-error layer
    // (`tc_p2_005_is_transient_table::lock_poisoned`). Anyone touching
    // the classification policy or this branch must reconfirm by hand.
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
            // Narrow the error to its rusqlite source per D-9 — only
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
}

/// ATOM-007 (N-2): when a `Manual`-mode persister is dropped while
/// dirty wallets remain, log a structured `tracing::error!` so the
/// silent-data-loss footgun (the buffer dies with the persister)
/// surfaces in operator logs.
///
/// We intentionally do NOT auto-flush from `Drop` — `flush_inner`
/// can fail and `Drop` cannot propagate errors, so a swallow there
/// would be a worse failure mode than the loud log. `Immediate`-mode
/// persisters are durable on every `store` so they never trip this.
impl Drop for SqlitePersister {
    fn drop(&mut self) {
        if !matches!(self.config.flush_mode, FlushMode::Manual) {
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
    /// **Query budget (FR-P4-6).** Constant-query w.r.t. wallet count:
    /// one `SELECT` over `wallet_metadata` for the wallet-id list, then
    /// per-wallet sync-header + count reads bounded by that list.
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
        let conn = self.conn().map_err(PersistenceError::from)?;
        let mut state = ClientStartState::default();

        let addrs_all = schema::platform_addrs::load_all(&conn).map_err(PersistenceError::from)?;
        let wallets_seen = addrs_all.len();
        let mut addresses_loaded: usize = 0;

        for (wallet_id, (addrs, count)) in addrs_all {
            if count > 0
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
    // Probe writability via `tempfile::NamedTempFile` — unguessable
    // name, no race against concurrent persister opens (CODE-008).
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
    let table_exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
            [],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !table_exists {
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
