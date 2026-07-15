//! [`SqlitePersister`] — the canonical `PlatformWalletPersistence` impl.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use rusqlite::{Connection, OptionalExtension};

use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
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
use crate::sqlite::util::wallet::{apply_persisted_core_state, build_wallet};

/// Persisted-but-not-rehydrated areas, surfaced in the structured
/// `tracing::info!` summary on every `load()`.
///
/// - `token_balances`: written by the `token_balances` slot but not read
///   back by `load()` (no reader wired in yet).
/// - `dashpay::overlay`: the `dashpay_profiles` /
///   `dashpay_payments_overlay` tables are a write-only indexed overlay;
///   DashPay state rehydrates from the identities blob, not these tables.
pub(crate) const LOAD_UNIMPLEMENTED: &[&str] = &["token_balances", "dashpay::overlay"];

/// Outcome of a `prune_backups` call.
///
/// Invariant: `kept == total_eligible - removed.len()`; a file is `kept`
/// if the policy retained it OR `remove_file` failed (so `failed_removals`
/// is a subset of `kept`). Either way it's still on disk.
#[derive(Debug)]
pub struct PruneReport {
    /// Unlinked paths, oldest-first by filename timestamp.
    pub removed: Vec<PathBuf>,
    /// Count still on disk (`total_eligible - removed.len()`), including
    /// every `failed_removals` entry.
    pub kept: usize,
    /// Files we couldn't remove, paired with the `io::Error`. Returned in
    /// `Ok(report)` so the caller can re-invoke to retry the stragglers.
    pub failed_removals: Vec<(PathBuf, std::io::Error)>,
}

/// Retention policy for `prune_backups`.
///
/// `keep_last_n` is a **floor**: the N newest backups are always kept even
/// if `max_age` would evict them, so a policy setting both can never delete
/// everything. `keep_last_n = None` gives no floor (age-only may prune
/// all); `default()` (both `None`) keeps every file.
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

/// Canonicalized paths held by a live [`SqlitePersister`] in this process.
/// Refusing a second in-process open ([`WalletStorageError::AlreadyOpen`])
/// prevents two handles with independent buffers diverging; cross-process
/// peers are handled by SQLite's own EXCLUSIVE locking.
fn open_path_registry() -> &'static Mutex<HashSet<PathBuf>> {
    static REGISTRY: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(HashSet::new()))
}

/// Insert `path`, returning [`WalletStorageError::AlreadyOpen`] if held.
/// Recover from a poisoned registry mutex rather than wedging every open.
fn register_open_path(path: PathBuf) -> Result<(), WalletStorageError> {
    let mut set = open_path_registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if set.contains(&path) {
        return Err(WalletStorageError::AlreadyOpen { path });
    }
    set.insert(path);
    Ok(())
}

/// Remove `path` from the open-path registry on persister drop.
fn release_open_path(path: &Path) {
    let mut set = open_path_registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    set.remove(path);
}

/// `true` if `path` is held open by a live [`SqlitePersister`] in this
/// process. Callers pass a canonicalized path (matching how `open()`
/// registers it).
fn is_path_open(path: &Path) -> bool {
    open_path_registry()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .contains(path)
}

/// SQLite-backed `PlatformWalletPersistence`.
pub struct SqlitePersister {
    config: SqlitePersisterConfig,
    /// Canonicalized DB path held in the process-wide open-path registry.
    /// Removed from the registry when this persister drops.
    registered_path: PathBuf,
    // Single connection serializes reads through the write lock —
    // acceptable for the current per-wallet workload; a read-only pool is
    // the planned follow-up if read contention becomes measurable.
    conn: Arc<Mutex<Connection>>,
    buffer: Buffer,
    /// Test-only one-shot injector for `flush_inner`.
    #[cfg(any(test, feature = "__test-helpers"))]
    primed_flush_error: Mutex<Option<WalletStorageError>>,
    /// Test-only one-shot injector for `delete_wallet`'s pre-flush phase.
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
        // Log every open failure where it surfaces — this is the crate's
        // highest-stakes boundary (on-disk corruption, forward-incompatible
        // schema, mid-run migration failure, in-process double-open) and the
        // caller only sees the returned `Err`, not why it happened.
        // `AlreadyOpen` is the one benign race (the loser retries once the
        // winner drops), so it warns rather than errors.
        let path = config.path.clone();
        Self::open_inner(config).inspect_err(|e| match e {
            WalletStorageError::AlreadyOpen { .. } => tracing::warn!(
                path = %path.display(),
                error_kind = e.error_kind_str(),
                "SqlitePersister open refused: database already open in this process"
            ),
            _ => tracing::error!(
                path = %path.display(),
                error_kind = e.error_kind_str(),
                error = %e,
                "SqlitePersister failed to open database"
            ),
        })
    }

    fn open_inner(config: SqlitePersisterConfig) -> Result<Self, WalletStorageError> {
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

        // Pre-create owner-only (0600) with O_EXCL before rusqlite opens:
        // no umask window, and a planted symlink makes the create fail
        // rather than redirect (no chmod-by-path TOCTOU). No-op if it
        // already exists.
        precreate_secure(&config.path)?;

        // Open + apply pragmas before checking pending migrations so the
        // integrity probe sees the configured journal mode / busy timeout.
        let mut conn =
            crate::sqlite::conn::open_conn(&config.path, crate::sqlite::conn::Access::ReadWrite)?;
        // Re-tighten to 0600 and sweep the WAL/SHM sidecars SQLite created.
        apply_secure_permissions(&config.path)?;
        apply_pragmas(&mut conn, &config)?;

        // `schema_history` presence is the pre-existing-vs-brand-new
        // signal; query errors propagate rather than masking as "none".
        let had_schema_history = crate::sqlite::migrations::has_schema_history(&conn)?;
        // Integrity-check a pre-existing DB BEFORE migrations alter it,
        // else a corrupt DB gets backed up and migrated in one pass,
        // making the pre-migration auto-backup useless for rollback.
        if had_schema_history {
            crate::sqlite::backup::run_integrity_check(&conn, |report| {
                WalletStorageError::IntegrityCheckFailed { report }
            })?;
        }
        // Refuse a newer-binary DB: refinery's run() no-ops at
        // pending==0, after which blob decoders would read forward-schema
        // bytes. Then assert the wallet application_id and a well-formed
        // schema_history BEFORE refinery, so a foreign or
        // corrupted-but-integrity-valid DB fails typed instead of being
        // migrated in place or panicking the runner.
        if had_schema_history {
            crate::sqlite::migrations::assert_schema_version_supported(&conn)?;
            crate::sqlite::conn::assert_wallet_application_id(&conn)?;
            crate::sqlite::migrations::assert_schema_history_well_formed(&conn)?;
        } else if crate::sqlite::migrations::db_has_objects(&conn)? {
            // A pre-existing file with schema objects but NO refinery history is
            // a foreign (non-wallet) SQLite DB. Migrating it in place would graft
            // wallet tables onto someone else's schema; reject via the
            // application_id gate (a foreign DB never carries our magic) instead.
            crate::sqlite::conn::assert_wallet_application_id(&conn)?;
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

        let _report = crate::sqlite::migrations::run_for_open(&mut conn)?;

        // Claim the path LAST so a failed open leaves no stale claim;
        // canonicalize so symlinks / `.`-segments key the same as a
        // sibling open would.
        let registered_path = config
            .path
            .canonicalize()
            .unwrap_or_else(|_| config.path.clone());
        register_open_path(registered_path.clone())?;

        Ok(Self {
            config,
            registered_path,
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
    /// The pre-restore auto-backup is taken BEFORE the restore body's
    /// `BEGIN EXCLUSIVE`, so under concurrent cross-process access the
    /// rollback point may miss writes a peer committed in between. Callers
    /// must serialize restore intent across processes.
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
        // Refuse to overwrite a database a live persister in this process is
        // still holding open: that handle's buffer/connection would silently
        // diverge from the restored bytes. Canonicalize to match how `open()`
        // registers the path (symlinks / `.`-segments resolve to one key); a
        // not-yet-existing dest can't be open, so the fallback path is fine.
        let dest_canonical = dest_db_path
            .canonicalize()
            .unwrap_or_else(|_| dest_db_path.to_path_buf());
        if is_path_open(&dest_canonical) {
            return Err(WalletStorageError::AlreadyOpen {
                path: dest_canonical,
            });
        }
        if !skip_backup && dest_db_path.exists() {
            let dir = auto_backup_dir.ok_or(WalletStorageError::AutoBackupDisabled {
                operation: AutoBackupOperation::Restore,
            })?;
            // Open read-only just long enough to snapshot under auto_backup_dir.
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
        // No row-count fingerprint guards the snapshot→EXCLUSIVE window:
        // `backup::restore_from`'s `BEGIN EXCLUSIVE` covers the body, and a
        // count would miss in-place UPDATEs and give false confidence.
        // Callers needing a quiesced point serialize restore intent.
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
    /// The pre-delete auto-backup is taken BEFORE the cascade's
    /// `BEGIN EXCLUSIVE`, so under concurrent cross-process access the
    /// rollback point may miss writes a peer committed in between. Callers
    /// must serialize delete intent across processes.
    ///
    /// # Racing stores
    ///
    /// A `store(wallet_id, ...)` racing this call is **discarded** after
    /// the delete commits — it may return `Ok(())` (Manual mode buffers
    /// it) but a post-commit re-drain removes it. Synchronize at the
    /// caller layer if you need other semantics.
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
        // Take the conn mutex first so in-process `store()` blocks;
        // cross-process peers are excluded by `BEGIN EXCLUSIVE` below.
        let mut conn = self.conn()?;

        // Drain the buffer so a later flush can't resurrect the wallet and
        // so a buffer-only wallet still counts as existing. Held in
        // `drained_slot` and consumed only after commit.
        let drained = self.buffer.take_for_flush(&wallet_id)?;
        let had_buffered = drained.is_some();
        let drained_slot: std::cell::Cell<Option<PlatformWalletChangeSet>> =
            std::cell::Cell::new(drained);

        // Any pre-commit failure must restore the changeset so a delete
        // that didn't happen doesn't lose pending writes.
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
            // Existence check before backup so we don't snapshot for an
            // unknown wallet.
            let exists_pre_flush = conn
                .query_row(
                    "SELECT 1 FROM wallets WHERE wallet_id = ?1",
                    rusqlite::params![wallet_id.as_slice()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !had_buffered && !exists_pre_flush {
                return Err(WalletStorageError::WalletNotFound { wallet_id });
            }

            // Test-only injector to fail the pre-flush below.
            #[cfg(any(test, feature = "__test-helpers"))]
            let primed_pre_flush_error = self.consume_primed_pre_flush_error();

            // Flush the drained buffer (its own EXCLUSIVE tx) BEFORE
            // `run_auto_backup` so the snapshot includes pending writes;
            // otherwise rollback-from-backup can't recover them. The backup
            // must precede the cascade's `BEGIN EXCLUSIVE` because
            // `Backup::new` deadlocks if the source holds an active write tx.
            if let Some(cs) = drained_slot.take() {
                #[cfg(any(test, feature = "__test-helpers"))]
                if let Some(primed) = primed_pre_flush_error {
                    drained_slot.set(Some(cs));
                    return Err(primed);
                }
                let pre_flush_tx = match conn
                    .transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)
                {
                    Ok(tx) => tx,
                    Err(e) => {
                        drained_slot.set(Some(cs));
                        return Err(WalletStorageError::Sqlite(e));
                    }
                };
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

            // EXCLUSIVE for the cascade window excludes cross-process peers
            // that the in-process conn mutex can't; they back off via
            // `busy_timeout`.
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Exclusive)?;

            // Deleting the parent `wallets` row drives all cleanup: native
            // `ON DELETE CASCADE` clears FK-bearing tables and AFTER DELETE
            // triggers reap the `meta_*` rows (the completeness test
            // asserts nothing survives).
            crate::sqlite::schema::wallets::delete(&tx, &wallet_id)?;
            tx.commit()?;
            drop(drained_slot.take());
            // Discard any changeset a Manual-mode store buffered during the
            // delete window — the wallet is gone.
            match self.buffer.take_for_flush(&wallet_id) {
                Ok(Some(_late)) => tracing::warn!(
                    wallet_id = %hex::encode(wallet_id),
                    "discarded racing buffered changeset after delete_wallet commit"
                ),
                Ok(None) => {}
                // The delete itself already committed, so this still returns
                // Ok — but a poisoned buffer mutex here is a signal every
                // later store/flush on this persister will hit, so log it at
                // the same level as the file's other LockPoisoned sites.
                Err(e) => tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error_kind = e.error_kind_str(),
                    "buffer mutex poisoned draining racing changeset after delete_wallet commit"
                ),
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

    /// Flush every dirty wallet regardless of flush mode — the only way
    /// `Manual` writes become durable, and the retry path for transient
    /// `Immediate`-mode failures left in the buffer. "Durable" means across
    /// application crash (WAL + `synchronous=NORMAL`); use
    /// [`Synchronous::Full`](crate::Synchronous) for power-loss durability.
    ///
    /// Continues past per-wallet failures: each outcome lands on the
    /// [`CommitReport`] (`succeeded` / `failed`), and `still_pending` fills
    /// only when a `LockPoisoned` short-circuit skips the rest. Returns
    /// `Err` only when enumerating the dirty set itself fails.
    pub fn commit_writes(&self) -> Result<CommitReport, PersistenceError> {
        self.commit_writes_inner()
    }

    fn commit_writes_inner(&self) -> Result<CommitReport, PersistenceError> {
        let mut report = CommitReport {
            succeeded: Vec::new(),
            failed: Vec::new(),
            still_pending: Vec::new(),
        };
        // Even in `Immediate` mode the buffer can be non-empty: a transient
        // `store()` failure re-merges the changeset, and only this drains
        // it regardless of flush mode.
        let dirty = self
            .buffer
            .dirty_wallets()
            .map_err(PersistenceError::from)?;
        let mut iter = dirty.into_iter();
        while let Some(id) = iter.next() {
            match self.flush_inner(&id) {
                Ok(()) => report.succeeded.push(id),
                Err(PersistenceError::LockPoisoned) => {
                    // Mutex is gone; record this as failed and the rest as
                    // never-attempted instead of hammering them.
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

    // The `__test-helpers` feature uses Cargo's `__` prefix convention:
    // not public API, downstream MUST NOT enable it.
    /// Test-only: borrow the write connection to seed rows or probe
    /// non-public tables/pragmas. Downstream MUST NOT enable the feature.
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

        // Test-only injector: surface a primed failure without touching SQL.
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
    /// surface as `FlushRetryable`; everything else drops the changeset
    /// and returns the original variant.
    //
    // TODO(qa): the fatal `LockPoisoned` branch has no e2e mutex-poison
    // test; verified by hand — reconfirm if you touch the classification.
    fn handle_flush_error(
        &self,
        wallet_id: &WalletId,
        cs: PlatformWalletChangeSet,
        err: WalletStorageError,
    ) -> Result<(), PersistenceError> {
        let field_count = populated_field_count(&cs);
        let kind = err.error_kind_str();
        if err.is_transient() {
            // A failed restore loses the changeset — itself fatal, so
            // surface it instead of the transient signal.
            if let Err(restore_err) = self.buffer.restore(*wallet_id, cs) {
                tracing::error!(
                    wallet_id = %hex::encode(wallet_id),
                    error_kind = restore_err.error_kind_str(),
                    restored_field_count = field_count,
                    "buffer restore failed after transient flush error — changeset lost"
                );
                return Err(PersistenceError::from(restore_err));
            }
            // Narrow to the rusqlite source for `FlushRetryable`.
            let source = match err {
                WalletStorageError::Sqlite(rusq) => rusq,
                WalletStorageError::FlushRetryable { source, .. } => source,
                other => {
                    // Defensive: "transient" but non-rusqlite source —
                    // surface raw rather than mislabel the source type.
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
            drop(cs);
            Err(PersistenceError::from(err))
        }
    }

    /// Test-only: arm a one-shot injection for the next `flush_inner`,
    /// for tests that care only how the wrapper reacts to the error.
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
    /// `delete_wallet`; fires only when there's a drained changeset to flush.
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

    /// Test-only: whether the wallet has a buffered changeset (asserts the
    /// buffer survives a failed pre-flush without consuming it).
    #[doc(hidden)]
    #[cfg(any(test, feature = "__test-helpers"))]
    pub fn buffer_has_changeset_for_test(&self, wallet_id: &WalletId) -> bool {
        self.buffer
            .dirty_wallets()
            .map(|v| v.iter().any(|w| w == wallet_id))
            .unwrap_or(false)
    }
}

/// On drop of a `Manual`-mode persister with dirty wallets, log an error
/// so the silent-data-loss footgun surfaces. We do NOT auto-flush from
/// `Drop`: `flush_inner` can fail and `Drop` can't propagate, so swallowing
/// would be worse than a loud log. `Immediate` mode never trips this.
impl Drop for SqlitePersister {
    fn drop(&mut self) {
        // Release the path claim FIRST so it happens regardless of flush
        // mode (the warning below early-returns for Immediate).
        release_open_path(&self.registered_path);
        if self.config.flush_mode != FlushMode::Manual {
            return;
        }
        // `dirty_wallets` only fails on a poisoned buffer mutex; surface
        // the lost state where we can.
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
        // `take_for_flush` drains the buffer — intentional in `Drop`: no
        // future caller can observe it, and we need the changeset to count
        // fields for the diagnostic.
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
    /// Merge `changeset` into the per-wallet buffer.
    ///
    /// Durability matrix:
    /// - [`FlushMode::Immediate`]: on `Ok`, durable across application
    ///   crash — one transaction wraps every per-table apply (all-or-
    ///   nothing). A transient failure restores the buffer and surfaces
    ///   [`WalletStorageError::FlushRetryable`]. Use
    ///   [`Synchronous::Full`](crate::Synchronous) for power-loss durability.
    /// - [`FlushMode::Manual`]: only merges into the buffer; durability
    ///   needs [`flush`](Self::flush) or
    ///   [`commit_writes`](Self::commit_writes).
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
    /// Populates `platform_addresses` and the keyless per-wallet `wallets`
    /// payload (network, birth height, account manifest, core state,
    /// identities, `Consumed`-filtered asset locks). Carries **no** `Wallet`
    /// or key material — the manager rebuilds each wallet watch-only and
    /// signs later on demand. The `tracing::info!` summary reports
    /// `wallets_rehydrated`.
    ///
    /// Fail-hard: any row that fails to decode (or has a malformed
    /// `wallet_id`) aborts the whole load — corruption is never skipped.
    ///
    /// **Query budget.** Platform addresses load via grouped bulk scans
    /// (constant), but the keyless per-wallet payload is a fan-out: one
    /// id-list `SELECT` plus a fixed set of per-wallet reads for each wallet
    /// (core state, identities, asset locks, contacts, identity keys, used
    /// addresses). O(wallets) queries overall — acceptable for one-shot
    /// startup, not the hot path.
    ///
    /// # Concurrency
    ///
    /// Holds the connection mutex for the whole read, so concurrent
    /// `store` / `flush` / `delete_wallet` block until it returns. Intended
    /// for one-shot startup use, not the hot write path.
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
        let mut addresses_loaded: usize = 0;
        for (wallet_id, (addrs, count)) in addrs_all {
            // Skip a wallet with no platform state at all (no addresses,
            // no registrations, all sync watermarks zero).
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

        // Per-wallet keyless rehydration payload; the manager rebuilds each
        // wallet watch-only and derives signing keys later on demand.
        let wallet_ids = schema::wallets::list_ids(&conn).map_err(PersistenceError::from)?;
        let wallets_seen = wallet_ids.len();
        for wallet_id in wallet_ids {
            let (network_str, birth_height) = schema::wallets::fetch(&conn, &wallet_id)
                .map_err(PersistenceError::from)?
                .ok_or_else(|| {
                    PersistenceError::backend(format!(
                        "wallets row vanished mid-load for {}",
                        hex::encode(wallet_id)
                    ))
                })?;
            let network = schema::wallets::parse_network(&network_str).ok_or_else(|| {
                PersistenceError::backend(format!(
                    "unknown persisted network {:?} for wallet {}",
                    network_str,
                    hex::encode(wallet_id)
                ))
            })?;

            let account_manifest =
                schema::accounts::load_state(&conn, &wallet_id).map_err(PersistenceError::from)?;
            let (core_state, utxo_accounts) =
                schema::core_state::load_state(&conn, &wallet_id, network)
                    .map_err(PersistenceError::from)?;
            // Pre-keyed rehydration: each `ManagedIdentity` leaves the loader
            // already carrying its own public keys + contact state (matching
            // the FFI persister), so signing works immediately post-load
            // without a key sync. `ClientWalletStartState.contacts` /
            // `.identity_keys` stay empty — nothing is layered on afterwards.
            let identity_manager = schema::identities::load_prekeyed(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            let unused_asset_locks = schema::asset_locks::load_unconsumed(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            // Used addresses drive the reuse guard: a used-then-emptied
            // address must never be handed back as a fresh receive address,
            // and must come back used on ITS OWN account so it is never
            // re-issued as a fresh receive address from that account. Union
            // the verbatim `core_address_pool` used-set (known owner) with the
            // `core_utxos`-derived set (spent + unspent; owner resolved per
            // script, `None` when no pool row covers it). The guard is
            // monotonic, so a mixed store — historical UTXOs plus a later
            // partial pool snapshot that never enumerates them — must surface
            // both; neither source may shadow the other. Keyed by address; the
            // pool source is authoritative on owner, so a `None` from the
            // `core_utxos` source never overrides a resolved pool owner. Two
            // resolved-but-disagreeing owners for one script means DB drift —
            // keep the pool owner and warn rather than crash rehydration.
            let used_core_addresses = {
                let mut union: std::collections::HashMap<
                    dashcore::Address,
                    Option<schema::core_pool::OwningAccount>,
                > = std::collections::HashMap::new();
                let pool = schema::core_pool::load_used_addresses(&conn, &wallet_id, network)
                    .map_err(PersistenceError::from)?;
                for (addr, owner) in pool {
                    union.entry(addr).or_insert(Some(owner));
                }
                let utxo = schema::core_state::load_used_addresses(&conn, &wallet_id, network)
                    .map_err(PersistenceError::from)?;
                for (addr, owner) in utxo {
                    match union.entry(addr) {
                        std::collections::hash_map::Entry::Occupied(existing) => {
                            if let (Some(pool_owner), Some(utxo_owner)) = (existing.get(), &owner) {
                                if pool_owner != utxo_owner {
                                    tracing::warn!(
                                        wallet_id = %hex::encode(wallet_id),
                                        pool_owner = %format!(
                                            "{}[{}]",
                                            pool_owner.account_type, pool_owner.account_index
                                        ),
                                        utxo_owner = %format!(
                                            "{}[{}]",
                                            utxo_owner.account_type, utxo_owner.account_index
                                        ),
                                        "rehydration: used address resolves to different owning \
                                         accounts in core_address_pool vs core_utxos — keeping the \
                                         pool owner (authoritative); likely store drift"
                                    );
                                }
                            }
                        }
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            slot.insert(owner);
                        }
                    }
                }
                union
            };

            // Reconstruct a populated `ManagedWalletInfo` from typed rows:
            // rebuild the wallet watch-only from the manifest, then layer the
            // persisted core-state projection (UTXOs, sync watermarks,
            // chainlock, used-address pool depth) onto it. The manager consumes
            // this directly — the old skeleton + core_state replay fallback is
            // gone.
            let wallet = if account_manifest.is_empty() {
                // No core (spending) accounts for this wallet. An empty manifest
                // is NOT necessarily an orphaned row: a platform-only wallet — a
                // Platform identity plus contacts, with no core accounts —
                // legitimately has one. Register it as an external-signable
                // placeholder (empty AccountCollection) that still carries its
                // platform-side state (identities, contacts); the manager
                // registers it like any other wallet. The genuinely-orphaned
                // case (a crash between the wallet-row write and the first
                // account write) also lands here and is harmless — it rehydrates
                // as an empty wallet.
                //
                // TODO(product decision needed, task #14): the orphaned variant
                // leaves a permanently empty manifest. It is not corrupted or
                // lost, but there is no recovery path today: no re-registration
                // flow, no eviction, no surfacing to the user. Open question:
                // does this need one (a TTL-based cleanup, a re-registration
                // entry point, or a surfaced "orphaned wallet" diagnostic), or is
                // register-empty-forever acceptable? Awaiting product decision;
                // not addressed here.
                key_wallet::wallet::Wallet::new_external_signable(
                    network,
                    wallet_id,
                    key_wallet::account::account_collection::AccountCollection::new(),
                )
            } else {
                build_wallet(network, wallet_id, &account_manifest).map_err(|e| {
                    PersistenceError::backend(format!(
                        "watch-only wallet rebuild failed for {}: {e}",
                        hex::encode(wallet_id)
                    ))
                })?
            };
            let mut wallet_info =
                key_wallet::wallet::managed_wallet_info::ManagedWalletInfo::from_wallet(
                    &wallet,
                    birth_height,
                );
            apply_persisted_core_state(
                &mut wallet_info,
                &account_manifest,
                &core_state,
                &utxo_accounts,
                &used_core_addresses,
            )
            .map_err(|e| {
                PersistenceError::backend(format!(
                    "core-state rehydration failed for {}: {e}",
                    hex::encode(wallet_id)
                ))
            })?;

            state.wallets.insert(
                wallet_id,
                platform_wallet::changeset::ClientWalletStartState {
                    wallet,
                    wallet_info,
                    identity_manager,
                    unused_asset_locks,
                },
            );
        }
        let wallets_rehydrated = state.wallets.len();

        tracing::info!(
            wallets_seen,
            addresses_loaded,
            wallets_rehydrated,
            wallets_pending_rehydration = 0usize,
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

/// Count of top-level changeset slots carrying data, for the
/// `restored_field_count` / `dropped_field_count` tracing fields. Computed
/// from the public fields so no storage-only helper leaks into the
/// `rs-platform-wallet` API.
fn populated_field_count(cs: &PlatformWalletChangeSet) -> usize {
    // Single source of truth with the version-domain mapping: each populated
    // field is exactly one touched domain.
    schema::versions::touched_domains(cs).len()
}

fn validate_config(config: &SqlitePersisterConfig) -> Result<(), WalletStorageError> {
    if config.synchronous == Synchronous::Off {
        return Err(WalletStorageError::ConfigInvalid {
            reason: "synchronous=Off is rejected (data-loss footgun)",
        });
    }
    // `journal_mode` Memory/Off keeps no on-disk rollback journal, making
    // a wallet DB crash-unsafe — reject loudly.
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
    // `busy_timeout=0` makes contended writers fail-fast with BUSY;
    // warn (not reject) since a few tests legitimately want that.
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
    // `foreign_keys` is enabled + read-back-asserted in `open_conn`.
    conn.pragma_update(None, "journal_mode", config.journal_mode.pragma_value())?;
    // Read `journal_mode` back: `pragma_update` doesn't error when SQLite
    // silently falls back (e.g. WAL→DELETE on FUSE), which with
    // synchronous=NORMAL risks corruption on power loss.
    let applied_journal: String =
        conn.pragma_query_value(None, "journal_mode", |row| row.get(0))?;
    if !applied_journal.eq_ignore_ascii_case(config.journal_mode.pragma_value()) {
        return Err(WalletStorageError::JournalModeNotApplied {
            requested: config.journal_mode.pragma_value(),
            actual: applied_journal,
        });
    }
    conn.pragma_update(None, "synchronous", config.synchronous.pragma_value())?;
    let ms = safe_cast::u64_to_i64(
        "busy_timeout_ms",
        u64::try_from(config.busy_timeout.as_millis()).unwrap_or(i64::MAX as u64),
    )?;
    conn.pragma_update(None, "busy_timeout", ms)?;
    Ok(())
}

/// Apply every populated sub-changeset of `cs` against `tx` without
/// committing (caller owns the tx). Separate from
/// `write_changeset_in_one_tx` so `delete_wallet_inner` can flush a drained
/// buffer into its own pre-delete tx.
fn apply_changeset_to_tx(
    tx: &rusqlite::Transaction<'_>,
    wallet_id: &WalletId,
    cs: &PlatformWalletChangeSet,
) -> Result<(), WalletStorageError> {
    if let Some(meta) = cs.wallet_metadata.as_ref() {
        schema::wallets::upsert(tx, wallet_id, meta)?;
    }
    if !cs.account_registrations.is_empty() {
        schema::accounts::apply_registrations(tx, wallet_id, &cs.account_registrations)?;
    }
    // Pools land before core so the UTXO writer can attribute each outpoint
    // to its owning account by matching the outpoint's script against a
    // freshly-written `core_address_pool` row.
    if !cs.account_address_pools.is_empty() {
        schema::core_pool::apply_pools(tx, wallet_id, &cs.account_address_pools)?;
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
    // Bump each touched domain's version inside this same tx so a domain's
    // cache-invalidation marker commits atomically with its data.
    schema::versions::bump_touched_domains(tx, wallet_id, cs)?;
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
    // Fast-fail writability probe. TOCTOU by construction (the dir can flip
    // before `run_to`), but the real write has its own error path, so the
    // worst case is a later typed error instead of this early one.
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
