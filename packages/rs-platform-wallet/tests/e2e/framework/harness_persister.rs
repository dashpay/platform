//! The harness manager's persister: a per-process SQLite store the tests
//! can observe.
//!
//! Every changeset is committed to a private SQLite store
//! ([`SqlitePersister`]) before the call returns, and a failed commit is
//! returned to the wallet. Toward the wallet it attests only what that
//! store honours and the harness actually serves back:
//!
//! - [`ATOMIC_CHANGESETS`](PersistenceCapabilities::ATOMIC_CHANGESETS):
//!   the store applies each changeset in one SQLite transaction.
//! - [`SHIELDED_VIEWING_KEYS`](PersistenceCapabilities::SHIELDED_VIEWING_KEYS):
//!   Orchard FVKs are written to the store and `load` returns them. This
//!   is what `bind_shielded` requires.
//!
//! Everything else stays session-scoped, as with `NoPlatformPersistence`:
//! `load` returns no wallets (no `WALLET_RESTORE`), and
//! `get_core_tx_record` answers `None`, so the wallet's Core, identity and
//! asset-lock paths are the non-durable ones. Each bit is claimed only when
//! the store attests it too, and nothing is claimed if it failed to open.
//!
//! For the tests it also:
//!
//! - captures the wallet-level Core records the manager stores
//!   (`CoreChangeSet::records`, the folded row a host displays), every one
//!   in store order, for the live accounting checks, and
//! - reopens a disk snapshot of the store to read a row back as a host
//!   would after a restart.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use dashcore::Txid;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::{
    ClientStartState, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
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
    /// Every wallet-level Core record stored, per `(wallet, txid)`, in
    /// store order.
    records: Mutex<BTreeMap<(WalletId, Txid), Vec<TransactionRecord>>>,
}

/// The capabilities the harness serves back to the wallet; see the module
/// docs for why each one is honoured.
const HONOURED: PersistenceCapabilities = PersistenceCapabilities::ATOMIC_CHANGESETS
    .union(PersistenceCapabilities::SHIELDED_VIEWING_KEYS);

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
    /// Open the store in a fresh private temporary directory. A store that
    /// fails to open leaves the persister non-durable (no capabilities,
    /// writes discarded), so reload checks fail with that reason instead of
    /// the suite failing to start.
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
        }
    }

    /// The latest wallet-level record stored for `txid` in `wallet_id`.
    pub fn live_record(&self, wallet_id: WalletId, txid: &Txid) -> Option<TransactionRecord> {
        self.records
            .lock()
            .expect("harness persister records poisoned")
            .get(&(wallet_id, *txid))
            .and_then(|history| history.last())
            .cloned()
    }

    /// Every wallet-level record stored for `txid` in `wallet_id`, oldest
    /// first.
    pub fn stored_records(&self, wallet_id: WalletId, txid: &Txid) -> Vec<TransactionRecord> {
        self.records
            .lock()
            .expect("harness persister records poisoned")
            .get(&(wallet_id, *txid))
            .cloned()
            .unwrap_or_default()
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
        reopened
            .get_core_tx_record(wallet_id, txid)
            .map_err(|e| format!("read row from reopened snapshot: {e}"))
    }
}

impl PlatformWalletPersistence for HarnessPersister {
    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        match &self.sqlite {
            Ok(store) => store.persistence_capabilities().intersection(HONOURED),
            Err(_) => PersistenceCapabilities::NONE,
        }
    }

    fn store_commits_inline(&self) -> bool {
        // Opened with `FlushMode::Immediate`: `store` commits before returning.
        self.sqlite.is_ok()
    }

    fn persists_durably(&self) -> bool {
        false
    }

    fn store(
        &self,
        wallet_id: WalletId,
        changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        let records: Vec<TransactionRecord> = changeset
            .core
            .as_ref()
            .map(|core| core.records.clone())
            .unwrap_or_default();
        // Commit first: a changeset the store rejected never happened.
        if let Ok(store) = &self.sqlite {
            store.store(wallet_id, changeset)?;
        }
        let mut captured = self
            .records
            .lock()
            .expect("harness persister records poisoned");
        for record in records {
            captured
                .entry((wallet_id, record.txid))
                .or_default()
                .push(record);
        }
        Ok(())
    }

    fn flush(&self, wallet_id: WalletId) -> Result<(), PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.flush(wallet_id),
            Err(_) => Ok(()),
        }
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        // Only the shielded viewing keys are served back (SHIELDED_VIEWING_KEYS);
        // wallets stay session-scoped because WALLET_RESTORE is not claimed.
        let shielded = match &self.sqlite {
            Ok(store) => store.load()?.shielded,
            Err(_) => Default::default(),
        };
        Ok(ClientStartState {
            shielded,
            ..ClientStartState::default()
        })
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
