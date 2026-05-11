//! [`SqlitePersister`] — the canonical `PlatformWalletPersistence` impl.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use rusqlite::Connection;

use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

use crate::backup::{self, BackupKind};
use crate::buffer::Buffer;
use crate::config::{FlushMode, SqlitePersisterConfig, Synchronous};
use crate::error::{AutoBackupOperation, SqlitePersisterError};
use crate::schema::{self, PER_WALLET_TABLES};

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
    /// Single write connection. Wrapped in a `Mutex` because rusqlite's
    /// `Connection` is `!Sync`. Reads also go through this connection
    /// today (`r2d2_sqlite` deferred per the plan).
    conn: Arc<Mutex<Connection>>,
    buffer: Buffer,
}

impl SqlitePersister {
    /// Open or create the SQLite DB at `config.path`. Applies pragmas,
    /// runs migrations, optionally takes a pre-migration auto-backup.
    pub fn open(config: SqlitePersisterConfig) -> Result<Self, SqlitePersisterError> {
        validate_config(&config)?;
        if let Some(parent) = config.path.parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                // Parent dir must exist — refuse silently creating it
                // to keep "bad path" errors typed (NFR-6).
                return Err(SqlitePersisterError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("database parent directory not found: {}", parent.display()),
                )));
            }
        }

        // Open the connection AND apply pragmas before checking for
        // pending migrations so the integrity probe sees the configured
        // journal mode and busy timeout.
        let mut conn = Connection::open(&config.path)?;
        apply_pragmas(&mut conn, &config)?;

        // Determine whether `schema_history` exists *before* we run
        // migrations — that's the signal for "is this DB pre-existing
        // or brand-new?" (FR-15 vs FR-16).
        let had_schema_history: bool = conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'refinery_schema_history'",
                [],
                |_| Ok(true),
            )
            .unwrap_or(false);
        let pending = crate::migrations::embedded_migrations();
        let pending_count = if had_schema_history {
            count_pending(&mut conn, &pending)?
        } else {
            pending.len()
        };

        if pending_count > 0 && had_schema_history {
            // Pre-migration auto-backup. If `auto_backup_dir` is `None`
            // we refuse outright (FR-18).
            let Some(dir) = config.auto_backup_dir.as_ref() else {
                return Err(SqlitePersisterError::AutoBackupDisabled {
                    operation: AutoBackupOperation::OpenMigration,
                });
            };
            ensure_dir(dir)?;
            let from = current_schema_version(&mut conn).unwrap_or(0);
            let to = pending.iter().map(|(v, _)| *v).max().unwrap_or(from);
            let dest = dir.join(backup::auto_backup_filename(BackupKind::PreMigration {
                from,
                to,
            }));
            backup::run_to(&conn, &dest)?;
        }

        // Apply migrations.
        let _report = crate::migrations::run(&mut conn).map_err(SqlitePersisterError::Migration)?;

        Ok(Self {
            config,
            conn: Arc::new(Mutex::new(conn)),
            buffer: Buffer::new(),
        })
    }

    /// Take a manual online backup. `dest` may be a directory (auto-
    /// named `wallet-<ts>.db`) or a full file path (must not pre-exist).
    pub fn backup_to(&self, dest: &Path) -> Result<PathBuf, SqlitePersisterError> {
        let resolved = if dest.is_dir() {
            dest.join(backup::manual_backup_filename())
        } else {
            if dest.exists() {
                return Err(SqlitePersisterError::BackupDestinationExists {
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
    pub fn restore_from(
        dest_db_path: &Path,
        src_backup: &Path,
    ) -> Result<(), SqlitePersisterError> {
        backup::restore_from(dest_db_path, src_backup)
    }

    /// Apply retention to a directory of `wallet-*.db` (and/or
    /// `pre-*-*.db`) files.
    pub fn prune_backups(
        &self,
        dir: &Path,
        policy: RetentionPolicy,
    ) -> Result<PruneReport, SqlitePersisterError> {
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
    ) -> Result<DeleteWalletReport, SqlitePersisterError> {
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
    ) -> Result<DeleteWalletReport, SqlitePersisterError> {
        self.delete_wallet_inner(wallet_id, true)
    }

    fn delete_wallet_inner(
        &self,
        wallet_id: WalletId,
        skip_backup: bool,
    ) -> Result<DeleteWalletReport, SqlitePersisterError> {
        // Existence check FIRST — refusing on an unknown wallet must
        // not waste a backup file.
        {
            let conn = self.conn()?;
            let exists: bool = conn
                .query_row(
                    "SELECT 1 FROM wallet_metadata WHERE wallet_id = ?1",
                    rusqlite::params![wallet_id.as_slice()],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            if !exists {
                return Err(SqlitePersisterError::WalletNotFound { wallet_id });
            }
        }
        let backup_path = if skip_backup {
            None
        } else {
            self.run_auto_backup(AutoBackupOperation::DeleteWallet, &wallet_id)?
        };
        let mut conn = self.conn()?;
        let tx = conn.transaction()?;
        let mut rows_removed_per_table = BTreeMap::new();
        for &table in PER_WALLET_TABLES {
            let n: i64 = tx
                .query_row(
                    &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                    rusqlite::params![wallet_id.as_slice()],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            rows_removed_per_table.insert(table, n as usize);
        }
        crate::schema::wallet_meta::delete(&tx, &wallet_id)?;
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
    ) -> Result<Vec<(&'static str, usize)>, SqlitePersisterError> {
        let conn = self.conn()?;
        let mut out = Vec::with_capacity(PER_WALLET_TABLES.len());
        for &table in PER_WALLET_TABLES {
            let n: i64 = match wallet_id {
                Some(id) => conn
                    .query_row(
                        &format!("SELECT COUNT(*) FROM {table} WHERE wallet_id = ?1"),
                        rusqlite::params![id.as_slice()],
                        |row| row.get(0),
                    )
                    .unwrap_or(0),
                None => conn
                    .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                        row.get(0)
                    })
                    .unwrap_or(0),
            };
            out.push((table, n as usize));
        }
        Ok(out)
    }

    /// Lock the write connection.
    pub(crate) fn conn(&self) -> Result<MutexGuard<'_, Connection>, SqlitePersisterError> {
        self.conn
            .lock()
            .map_err(|_| SqlitePersisterError::LockPoisoned)
    }

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

    /// Take a single auto-backup. Returns the path written, or `None`
    /// when the operation is the CLI fast-path that disables backup.
    fn run_auto_backup(
        &self,
        op: AutoBackupOperation,
        wallet_id: &WalletId,
    ) -> Result<Option<PathBuf>, SqlitePersisterError> {
        let Some(dir) = self.config.auto_backup_dir.as_ref() else {
            return Err(SqlitePersisterError::AutoBackupDisabled { operation: op });
        };
        ensure_dir(dir)?;
        let conn = self.conn()?;
        let dest = dir.join(match op {
            AutoBackupOperation::OpenMigration => unreachable!(
                "OpenMigration auto-backups are taken during `open`, not via run_auto_backup"
            ),
            AutoBackupOperation::DeleteWallet => {
                backup::auto_backup_filename(BackupKind::PreDelete {
                    wallet_id: *wallet_id,
                })
            }
        });
        backup::run_to(&conn, &dest)?;
        Ok(Some(dest))
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
            .map_err(|e| PersistenceError::Backend(format!("failed to begin transaction: {e}")))?;
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
            .map_err(|e| PersistenceError::Backend(format!("commit failed: {e}")))?;
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

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        let conn = self.conn().map_err(PersistenceError::from)?;
        let mut state = ClientStartState::default();
        for wallet_id in schema::wallet_meta::list_ids(&conn).map_err(PersistenceError::from)? {
            let addrs = schema::platform_addrs::load_state(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            // Only include wallets with at least some platform-address
            // activity or sync state; otherwise the empty struct is
            // load-bearing noise.
            let count = schema::platform_addrs::count_per_wallet(&conn, &wallet_id)
                .map_err(PersistenceError::from)?;
            if count > 0 || addrs.sync_height > 0 || addrs.sync_timestamp > 0 {
                state.platform_addresses.insert(wallet_id, addrs);
            }
            // `wallets` reconstruction (full Wallet + ManagedWalletInfo)
            // requires xpub-driven rehydration that is out of scope for
            // this crate. The data is persisted in the schema; upstream
            // gains a constructor in a follow-up PR.
            // TODO(platform-wallet-sqlite): wire wallets[*] once
            // `Wallet::from_persisted` lands.
        }
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

fn validate_config(config: &SqlitePersisterConfig) -> Result<(), SqlitePersisterError> {
    if config.synchronous == Synchronous::Off {
        return Err(SqlitePersisterError::ConfigInvalid(
            "synchronous=Off is rejected (data-loss footgun)",
        ));
    }
    Ok(())
}

fn apply_pragmas(
    conn: &mut Connection,
    config: &SqlitePersisterConfig,
) -> Result<(), SqlitePersisterError> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", config.journal_mode.pragma_value())?;
    conn.pragma_update(None, "synchronous", config.synchronous.pragma_value())?;
    let ms = config.busy_timeout.as_millis().min(i64::MAX as u128) as i64;
    conn.pragma_update(None, "busy_timeout", ms)?;
    Ok(())
}

fn ensure_dir(dir: &Path) -> Result<(), SqlitePersisterError> {
    if !dir.exists() {
        std::fs::create_dir_all(dir).map_err(|source| {
            SqlitePersisterError::AutoBackupDirUnwritable {
                dir: dir.to_path_buf(),
                source,
            }
        })?;
    }
    // Probe writability with a sentinel that we immediately remove.
    let probe = dir.join(".platform-wallet-sqlite-write-probe");
    match std::fs::write(&probe, b"") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            Ok(())
        }
        Err(source) => Err(SqlitePersisterError::AutoBackupDirUnwritable {
            dir: dir.to_path_buf(),
            source,
        }),
    }
}

fn count_pending(
    conn: &mut Connection,
    embedded: &[(i32, String)],
) -> Result<usize, SqlitePersisterError> {
    let applied: std::collections::HashSet<i64> = {
        let mut stmt = conn
            .prepare("SELECT version FROM refinery_schema_history")
            .ok();
        match stmt.as_mut() {
            None => return Ok(embedded.len()),
            Some(stmt) => {
                let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
                rows.collect::<Result<_, _>>()?
            }
        }
    };
    Ok(embedded
        .iter()
        .filter(|(v, _)| !applied.contains(&(*v as i64)))
        .count())
}

fn current_schema_version(conn: &mut Connection) -> Option<i32> {
    conn.query_row(
        "SELECT MAX(version) FROM refinery_schema_history",
        [],
        |row| row.get::<_, Option<i64>>(0),
    )
    .ok()
    .flatten()
    .map(|v| v as i32)
}
