//! The harness manager's persister: a per-process SQLite store the tests
//! can observe.
//!
//! Every changeset is committed to a private SQLite store
//! ([`SqlitePersister`], [`FlushMode::Immediate`]) before the call returns,
//! and a failed commit is returned to the wallet. The persister is
//! transparent: every [`PlatformWalletPersistence`] method delegates to that
//! store, so the wallet sees exactly the capabilities the store attests
//! (including `WALLET_RESTORE` and `WALLET_DRAIN`) and drives its durable
//! Core, identity and asset-lock paths. A store that failed to open attests
//! nothing and discards writes, so the checks that need it fail with that
//! reason instead of the suite failing to start.
//!
//! The store lives in a fresh temporary directory per process, so `load`
//! at manager start returns nothing from earlier runs.
//!
//! For the tests it also:
//!
//! - captures the wallet-level Core records the manager stores
//!   (`CoreChangeSet::records`, the folded row a host displays), every one
//!   in store order, for the live accounting checks (the store keeps only
//!   the latest row per txid), and
//! - reopens a disk snapshot of the store to read state back as a host
//!   would after a restart.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::sync::Mutex;

use dashcore::Txid;
use dpp::prelude::Identifier;
use key_wallet::managed_account::transaction_record::TransactionRecord;
use platform_wallet::changeset::changeset::DpnsNameStateEntry;
use platform_wallet::changeset::traits::ListedCoreTxid;
use platform_wallet::changeset::{
    ClientStartState, PersistenceCapabilities, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::masternode::TrackedMasternode;
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::{FlushMode, SqlitePersister, SqlitePersisterConfig};
use tempfile::TempDir;

/// See the module docs.
pub struct HarnessPersister {
    /// The backing store, or why it could not be opened.
    sqlite: Result<SqlitePersister, String>,
    /// Private directory holding the store and its snapshots; removed on
    /// drop.
    dir: Option<TempDir>,
    /// Every wallet-level Core record stored, per `(wallet, txid)`, in
    /// store order.
    records: Mutex<BTreeMap<(WalletId, Txid), Vec<TransactionRecord>>>,
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
    /// Open the store in a fresh private temporary directory. A store that
    /// fails to open leaves the persister non-durable (no capabilities,
    /// writes discarded); see the module docs.
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
                "SQLite store unavailable; the wallet runs non-durable and reload checks will fail"
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
    /// store, as a host would after a restart.
    pub fn reloaded_record(
        &self,
        wallet_id: WalletId,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, String> {
        self.reopened_snapshot(&txid.to_string())?
            .get_core_tx_record(wallet_id, txid)
            .map_err(|e| format!("read row from reopened snapshot: {e}"))
    }

    /// The full client start state a host would load from a freshly
    /// opened snapshot of the store after a restart.
    pub fn reloaded_start_state(&self) -> Result<ClientStartState, String> {
        self.reopened_snapshot("start-state")?
            .load()
            .map_err(|e| format!("load reopened snapshot: {e}"))
    }

    /// Snapshot the store to disk and open the copy. The live handle
    /// cannot be reopened in-process: `SqlitePersister::open` refuses a
    /// double open.
    fn reopened_snapshot(&self, label: &str) -> Result<SqlitePersister, String> {
        let store = self.backing_store()?;
        let dir = self.dir.as_ref().ok_or("SQLite directory unavailable")?;
        let snapshot = store
            .backup_to(
                &dir.path()
                    .join(format!("snapshot-{label}-{}.db", unique_suffix())),
            )
            .map_err(|e| format!("snapshot store: {e}"))?;
        SqlitePersister::open(SqlitePersisterConfig::new(&snapshot).with_auto_backup_dir(None))
            .map_err(|e| format!("reopen snapshot: {e}"))
    }

    fn backing_store(&self) -> Result<&SqlitePersister, String> {
        self.sqlite
            .as_ref()
            .map_err(|e| format!("SQLite store unavailable: {e}"))
    }
}

impl PlatformWalletPersistence for HarnessPersister {
    fn store_commits_inline(&self) -> bool {
        self.sqlite
            .as_ref()
            .is_ok_and(|store| store.store_commits_inline())
    }

    fn persistence_capabilities(&self) -> PersistenceCapabilities {
        match &self.sqlite {
            Ok(store) => store.persistence_capabilities(),
            Err(_) => PersistenceCapabilities::NONE,
        }
    }

    fn store_transient_is_reissuable(&self) -> bool {
        // The capture below runs only after a successful commit, so a failed
        // round leaves nothing behind here either.
        self.sqlite
            .as_ref()
            .is_ok_and(|store| store.store_transient_is_reissuable())
    }

    fn stores_dashpay_backfill(&self) -> bool {
        self.sqlite
            .as_ref()
            .is_ok_and(|store| store.stores_dashpay_backfill())
    }

    fn persists_durably(&self) -> bool {
        self.sqlite
            .as_ref()
            .is_ok_and(|store| store.persists_durably())
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
        let account_slices: Vec<String> = changeset
            .core
            .as_ref()
            .map(|core| {
                core.account_records
                    .iter()
                    .map(|r| {
                        format!(
                            "{}:{:?}:in={}:out={}:net={}",
                            r.txid,
                            r.account_type,
                            r.input_details.len(),
                            r.output_details.len(),
                            r.net_amount
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        // Commit first: a changeset the store rejected never happened.
        if let Ok(store) = &self.sqlite {
            store.store(wallet_id, changeset)?;
        }
        for record in &records {
            tracing::debug!(
                target: "platform_wallet::e2e::harness_persister",
                wallet_id = %hex::encode(wallet_id),
                txid = %record.txid,
                account = ?record.account_type,
                context = ?record.context,
                inputs = record.input_details.len(),
                outputs = record.output_details.len(),
                direction = ?record.direction,
                net = record.net_amount,
                fee = ?record.fee,
                ?account_slices,
                "stored wallet-level record"
            );
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

    fn persist_tracked_masternodes(
        &self,
        network: dashcore::Network,
        records: &[TrackedMasternode],
    ) -> Result<(), PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.persist_tracked_masternodes(network, records),
            Err(_) => Ok(()),
        }
    }

    fn load_tracked_masternodes(
        &self,
        network: dashcore::Network,
    ) -> Result<Vec<TrackedMasternode>, PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.load_tracked_masternodes(network),
            Err(_) => Ok(Vec::new()),
        }
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.load(),
            Err(_) => Ok(ClientStartState::default()),
        }
    }

    fn get_core_tx_record(
        &self,
        wallet_id: WalletId,
        txid: &Txid,
    ) -> Result<Option<TransactionRecord>, PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.get_core_tx_record(wallet_id, txid),
            Err(_) => Ok(None),
        }
    }

    fn list_wallet_core_txids(
        &self,
        wallet_id: WalletId,
    ) -> Result<Option<Vec<ListedCoreTxid>>, PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.list_wallet_core_txids(wallet_id),
            Err(_) => Ok(None),
        }
    }

    fn get_dpns_name_state(
        &self,
        wallet_id: WalletId,
        wallet_identity_id: &Identifier,
        normalized_label: &str,
    ) -> Result<Option<DpnsNameStateEntry>, PersistenceError> {
        match &self.sqlite {
            Ok(store) => store.get_dpns_name_state(wallet_id, wallet_identity_id, normalized_label),
            Err(_) => Ok(None),
        }
    }
}

/// Distinguishes repeated snapshots (`backup_to` refuses an existing
/// destination file).
fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}
