//! The harness manager's persister: non-durable to the wallet, but
//! observable by the tests.
//!
//! To the wallet it behaves exactly like `NoPlatformPersistence`: it
//! claims no persistence capabilities, `load` returns an empty start
//! state and `get_core_tx_record` answers `None`, so every wallet code
//! path the suite drives is the same as without it. On the side it:
//!
//! - captures the wallet-level Core records the manager stores
//!   (`CoreChangeSet::records`, the folded row a host displays), for the
//!   live accounting checks, and
//! - tees every changeset into a per-process SQLite store, so a case can
//!   reopen a disk snapshot and read a row back as a host would after a
//!   restart.
//!
//! SQLite write failures are logged and counted, never returned: the tee
//! must not change the suite's behaviour. A reload check that finds no row
//! reports the count.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::{
    ClientStartState, PersistenceError, PlatformWalletChangeSet, PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use tempfile::TempDir;

/// See the module docs.
pub struct HarnessPersister {
    /// The tee store, or why it could not be opened.
    sqlite: Result<SqlitePersister, String>,
    /// Private directory holding the tee store and its snapshots; removed
    /// on drop.
    dir: Option<TempDir>,
    records: Mutex<BTreeMap<(WalletId, Txid), TransactionRecord>>,
    sqlite_store_errors: AtomicU64,
}

/// A fresh owner-only temporary directory.
///
/// `SqlitePersister::open` refuses a database with a group- or
/// other-writable non-sticky ancestor, so SQLite stores cannot live under
/// the harness workdir (created under the process umask, often 0775).
pub fn private_temp_dir(prefix: &str) -> std::io::Result<TempDir> {
    tempfile::Builder::new()
        .prefix(prefix)
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
}

impl HarnessPersister {
    /// Open the tee store in a fresh private temporary directory. A store
    /// that fails to open leaves the tee disabled, so reload checks fail
    /// with that reason instead of the suite failing to start.
    pub fn open() -> Self {
        let (dir, sqlite) = match private_temp_dir("platform-wallet-e2e-history-") {
            Ok(dir) => {
                let sqlite = SqlitePersister::open(
                    SqlitePersisterConfig::new(dir.path().join("history.db"))
                        .with_flush_mode(FlushMode::Immediate)
                        .with_auto_backup_dir(None),
                )
                .map_err(|e| e.to_string());
                (Some(dir), sqlite)
            }
            Err(e) => (None, Err(format!("create private temp dir: {e}"))),
        };
        if let Err(error) = &sqlite {
            tracing::warn!(
                target: "platform_wallet::e2e::harness_persister",
                %error,
                "SQLite tee store unavailable; reload checks will fail"
            );
        }
        Self {
            sqlite,
            dir,
            records: Mutex::new(BTreeMap::new()),
            sqlite_store_errors: AtomicU64::new(0),
        }
    }

    /// The latest wallet-level record stored for `txid` in `wallet_id`.
    pub fn live_record(&self, wallet_id: WalletId, txid: &Txid) -> Option<TransactionRecord> {
        self.records
            .lock()
            .expect("harness persister records poisoned")
            .get(&(wallet_id, *txid))
            .cloned()
    }

    /// Read the row for `txid` back from a freshly opened snapshot of the
    /// tee store, as a host would after a restart.
    pub fn reloaded_record(
        &self,
        wallet_id: WalletId,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, String> {
        let store = self
            .sqlite
            .as_ref()
            .map_err(|e| format!("SQLite tee store unavailable: {e}"))?;
        let dir = self
            .dir
            .as_ref()
            .ok_or("SQLite tee directory unavailable")?;
        let snapshot = store
            .backup_to(
                &dir.path()
                    .join(format!("snapshot-{txid}-{}.db", unique_suffix())),
            )
            .map_err(|e| format!("snapshot tee store: {e}"))?;
        let reopened =
            SqlitePersister::open(SqlitePersisterConfig::new(&snapshot).with_auto_backup_dir(None))
                .map_err(|e| format!("reopen tee snapshot: {e}"))?;
        let row = reopened
            .get_core_tx_record(wallet_id, txid)
            .map_err(|e| format!("read row from reopened snapshot: {e}"))?;
        let errors = self.sqlite_store_errors.load(Ordering::Relaxed);
        if row.is_none() && errors > 0 {
            return Err(format!(
                "no row for {txid}; the tee store rejected {errors} changeset(s), see \
                 platform_wallet::e2e::harness_persister warnings"
            ));
        }
        Ok(row)
    }
}

impl PlatformWalletPersistence for HarnessPersister {
    fn persists_durably(&self) -> bool {
        false
    }

    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        if let Some(core) = &changeset.core {
            let mut records = self
                .records
                .lock()
                .expect("harness persister records poisoned");
            for record in &core.records {
                records.insert((wallet_id, record.txid), record.clone());
            }
        }
        if let Ok(store) = &self.sqlite {
            if let Err(error) = store.store(wallet_id, changeset) {
                self.sqlite_store_errors.fetch_add(1, Ordering::Relaxed);
                tracing::warn!(
                    target: "platform_wallet::e2e::harness_persister",
                    %error,
                    wallet_id = %hex::encode(wallet_id),
                    "SQLite tee store rejected a changeset"
                );
            }
        }
        Ok(())
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        if let Ok(store) = &self.sqlite {
            if let Err(error) = store.flush(wallet_id) {
                tracing::warn!(
                    target: "platform_wallet::e2e::harness_persister",
                    %error,
                    "SQLite tee flush failed"
                );
            }
        }
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// Distinguishes repeated snapshots of the same txid (`backup_to` refuses
/// an existing destination file).
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
