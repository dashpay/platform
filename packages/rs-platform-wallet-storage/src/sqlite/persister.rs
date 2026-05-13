//! [`SqlitePersister`] — the canonical `PlatformWalletPersistence` impl.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension};

use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
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
/// Surfaced via the [`WalletStorageError::LoadIncomplete`] variant
/// and a `tracing::warn!` whenever `load` returns.
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
        // journal mode and busy timeout.
        let mut conn = Connection::open(&config.path)?;
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
            let dest_conn = Connection::open_with_flags(
                dest_db_path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
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
        // Existence check FIRST — refusing on an unknown wallet must
        // not waste a backup file. `.optional()?` propagates real SQL
        // errors (busy / corrupt) instead of swallowing them.
        {
            let conn = self.conn()?;
            let exists = conn
                .query_row(
                    "SELECT 1 FROM wallet_metadata WHERE wallet_id = ?1",
                    rusqlite::params![wallet_id.as_slice()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !exists {
                return Err(WalletStorageError::WalletNotFound { wallet_id });
            }
        }
        let backup_path = if skip_backup {
            None
        } else {
            let conn = self.conn()?;
            run_auto_backup(
                &conn,
                self.config.auto_backup_dir.as_deref(),
                BackupKind::PreDelete { wallet_id },
                AutoBackupOperation::DeleteWallet,
            )?
        };
        let mut conn = self.conn()?;
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
        Ok(DeleteWalletReport {
            wallet_id,
            backup_path,
            rows_removed_per_table,
        })
    }

    /// In Manual mode: flush every dirty wallet. In Immediate mode: no-op.
    pub fn commit_writes(&self) -> Result<(), PersistenceError> {
        match self.config.flush_mode {
            FlushMode::Immediate => Ok(()),
            FlushMode::Manual => {
                let dirty = self
                    .buffer
                    .dirty_wallets()
                    .map_err(PersistenceError::from)?;
                for id in dirty {
                    self.flush_inner(&id)?;
                }
                Ok(())
            }
        }
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

    // INTENTIONAL(PROJ-005): downstream cannot meaningfully enable
    // test-helpers because the methods are
    // `#[cfg(any(test, feature = "test-helpers"))]`; the feature
    // exists only so this crate's own integration tests can pull
    // themselves in via dev-deps with the feature on. Naming
    // convention warning (Cargo convention is `__test-helpers`) is
    // acknowledged and not adopted — see Cargo.toml.
    /// Test-only: borrow the write connection.
    ///
    /// Tests use this to seed `wallet_metadata` rows directly, run
    /// SELECTs against tables that aren't part of the public surface,
    /// or probe `PRAGMA foreign_keys` / `PRAGMA journal_mode`. Gated
    /// behind `cfg(test)` and the `test-helpers` feature — downstream
    /// crates cannot reach it.
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn lock_conn_for_test(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().expect("conn mutex poisoned")
    }

    /// Test-only: read the resolved config. Same visibility rules as
    /// [`lock_conn_for_test`](Self::lock_conn_for_test).
    #[doc(hidden)]
    #[cfg(any(test, feature = "test-helpers"))]
    pub fn config_for_test(&self) -> &SqlitePersisterConfig {
        &self.config
    }

    fn flush_inner(&self, wallet_id: &WalletId) -> Result<(), PersistenceError> {
        let cs = self
            .buffer
            .drain(wallet_id)
            .map_err(PersistenceError::from)?;
        let Some(cs) = cs else { return Ok(()) };
        let mut conn = self.conn().map_err(PersistenceError::from)?;
        let tx = conn
            .transaction()
            .map_err(WalletStorageError::from)
            .map_err(PersistenceError::from)?;
        if let Some(meta) = cs.wallet_metadata.as_ref() {
            schema::wallet_meta::upsert(&tx, wallet_id, meta).map_err(PersistenceError::from)?;
        }
        if !cs.account_registrations.is_empty() {
            schema::accounts::apply_registrations(&tx, wallet_id, &cs.account_registrations)
                .map_err(PersistenceError::from)?;
        }
        if !cs.account_address_pools.is_empty() {
            schema::accounts::apply_pools(&tx, wallet_id, &cs.account_address_pools)
                .map_err(PersistenceError::from)?;
        }
        if let Some(core) = cs.core.as_ref() {
            schema::core_state::apply(&tx, wallet_id, core).map_err(PersistenceError::from)?;
        }
        if let Some(identities) = cs.identities.as_ref() {
            schema::identities::apply(&tx, wallet_id, identities)
                .map_err(PersistenceError::from)?;
        }
        if let Some(keys) = cs.identity_keys.as_ref() {
            schema::identity_keys::apply(&tx, wallet_id, keys).map_err(PersistenceError::from)?;
        }
        if let Some(contacts) = cs.contacts.as_ref() {
            schema::contacts::apply(&tx, wallet_id, contacts).map_err(PersistenceError::from)?;
        }
        if let Some(addrs) = cs.platform_addresses.as_ref() {
            schema::platform_addrs::apply(&tx, wallet_id, addrs).map_err(PersistenceError::from)?;
        }
        if let Some(locks) = cs.asset_locks.as_ref() {
            schema::asset_locks::apply(&tx, wallet_id, locks).map_err(PersistenceError::from)?;
        }
        if let Some(balances) = cs.token_balances.as_ref() {
            schema::token_balances::apply(&tx, wallet_id, balances)
                .map_err(PersistenceError::from)?;
        }
        if cs.dashpay_profiles.is_some() || cs.dashpay_payments_overlay.is_some() {
            schema::dashpay::apply(
                &tx,
                wallet_id,
                cs.dashpay_profiles.as_ref(),
                cs.dashpay_payments_overlay.as_ref(),
            )
            .map_err(PersistenceError::from)?;
        }
        tx.commit()
            .map_err(WalletStorageError::from)
            .map_err(PersistenceError::from)?;
        Ok(())
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
    /// **Partial reconstruction caveat.** Today the implementation
    /// populates `ClientStartState::platform_addresses` and leaves
    /// `ClientStartState::wallets` empty — the latter requires an
    /// upstream `Wallet::from_persisted` constructor that doesn't
    /// exist yet. The data IS persisted in the SQLite schema and is
    /// recoverable via direct queries; only the rehydrated
    /// `(Wallet, ManagedWalletInfo)` pair is unavailable.
    ///
    /// Callers needing the partial-completion signal as a typed
    /// value should call `inspect_counts` after a successful `load`
    /// — non-zero counts in non-empty start-state buckets indicate
    /// the sub-area is persisted but not yet reconstructed. The
    /// `LOAD_UNIMPLEMENTED` constant names the affected
    /// `ClientStartState` field paths.
    ///
    /// A `tracing::warn!` is emitted on every `load` call until the
    /// reconstruction lands.
    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        let conn = self.conn().map_err(PersistenceError::from)?;
        let mut state = ClientStartState::default();
        for wallet_id in schema::wallet_meta::list_ids(&conn).map_err(PersistenceError::from)? {
            let addrs = schema::platform_addrs::load_state(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            let count = schema::platform_addrs::count_per_wallet(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            if count > 0 || addrs.sync_height > 0 || addrs.sync_timestamp > 0 {
                state.platform_addresses.insert(wallet_id, addrs);
            }
        }
        tracing::warn!(
            unimplemented = ?LOAD_UNIMPLEMENTED,
            "load() returned a partial ClientStartState — see SqlitePersister::load rustdoc"
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
    conn.pragma_update(None, "foreign_keys", "ON")?;
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
