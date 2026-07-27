#![allow(clippy::field_reassign_with_default)]

//! `delete_wallet` and `commit_writes` are inherent `SqlitePersister`
//! methods (not trait methods), returning the storage crate's
//! `DeleteWalletReport` / `CommitReport`. The persister is still usable
//! behind `Arc<dyn PlatformWalletPersistence>` for `store`/`flush`/`load`.

mod common;

use std::sync::Arc;

use common::{ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, ro_conn, wid};
use platform_wallet::changeset::{
    ClientStartState, CoreChangeSet, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;
use platform_wallet_storage::FlushMode;

fn core_with_height(synced_height: u32, last_processed_height: u32) -> CoreChangeSet {
    CoreChangeSet {
        synced_height: Some(synced_height),
        last_processed_height: Some(last_processed_height),
        ..Default::default()
    }
}

fn changeset(core: CoreChangeSet) -> PlatformWalletChangeSet {
    PlatformWalletChangeSet {
        core: Some(core),
        ..Default::default()
    }
}

/// Stub persister that implements only the trait surface (`store` /
/// `flush` / `load`); used to prove the trait is object-safe and
/// dispatchable without a concrete backend type.
struct StoreOnlyPersister;

impl PlatformWalletPersistence for StoreOnlyPersister {
    fn store(
        &self,
        _wallet_id: WalletId,
        _changeset: PlatformWalletChangeSet,
    ) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn flush(&self, _wallet_id: WalletId) -> Result<(), PersistenceError> {
        Ok(())
    }

    fn load(&self) -> Result<ClientStartState, PersistenceError> {
        Ok(ClientStartState::default())
    }
}

/// The trait dispatches `store` / `flush` / `load` through
/// `Arc<dyn PlatformWalletPersistence>` without a concrete backend type.
#[test]
fn trait_object_dispatches_store_flush_load() {
    let persister: Arc<dyn PlatformWalletPersistence> = Arc::new(StoreOnlyPersister);
    let wallet_id = wid(0xAB);
    persister
        .store(wallet_id, PlatformWalletChangeSet::default())
        .expect("store");
    persister.flush(wallet_id).expect("flush");
    let state = persister.load().expect("load");
    assert!(state.wallets.is_empty());
}

/// The inherent `delete_wallet` cascades the on-disk rows and reports
/// the deleted id plus the auto-backup path it took.
#[test]
fn inherent_delete_wallet_cascades_rows() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x55);
    ensure_wallet_meta(&persister, &w);
    PlatformWalletPersistence::store(&persister, w, changeset(core_with_height(11, 11)))
        .expect("store must succeed in Immediate mode");

    let count_for = |id: &[u8; 32]| -> i64 {
        ro_conn(&path)
            .query_row(
                "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
                rusqlite::params![id.as_slice()],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(count_for(&w), 1);

    let report = persister
        .delete_wallet(w)
        .expect("delete_wallet must succeed");
    assert_eq!(report.wallet_id, w);
    assert!(
        report.backup_path.is_some(),
        "delete_wallet must take an auto-backup (safe-by-default)"
    );
    assert_eq!(count_for(&w), 0);
}

/// The inherent `commit_writes` flushes every dirty wallet.
#[test]
fn inherent_commit_writes_flushes_dirty() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0x11);
    let b = wid(0x22);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    PlatformWalletPersistence::store(&persister, a, changeset(core_with_height(3, 3)))
        .expect("store A");
    PlatformWalletPersistence::store(&persister, b, changeset(core_with_height(4, 4)))
        .expect("store B");

    let report = persister
        .commit_writes()
        .expect("commit_writes must succeed");
    assert!(report.is_ok(), "report={report:?}");
    assert_eq!(report.succeeded.len(), 2);

    let count_for = |id: &[u8; 32]| -> i64 {
        ro_conn(&path)
            .query_row(
                "SELECT COUNT(*) FROM core_sync_state WHERE wallet_id = ?1",
                rusqlite::params![id.as_slice()],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(count_for(&a), 1);
    assert_eq!(count_for(&b), 1);
}

/// SQLite attests durability in BOTH flush modes, so the fail-closed
/// `persists_durably` gate on invitation creation
/// (`create_invitation` refuses non-durable backends) accepts a
/// SQLite-backed wallet. Immediate mode is durable at `store`; Manual
/// mode at the explicit `flush` the gated flow performs before
/// anything irreversible. Checked through the trait object exactly as
/// the gate reads it. A trait-default stub stays `false` (fail-closed
/// baseline the gate relies on).
#[test]
fn sqlite_attests_durability_for_invitation_gate() {
    let (immediate, _tmp_a, _path_a) = fresh_persister();
    let (manual, _tmp_b, _path_b) = fresh_persister_with_mode(FlushMode::Manual);

    let immediate: Arc<dyn PlatformWalletPersistence> = Arc::new(immediate);
    let manual: Arc<dyn PlatformWalletPersistence> = Arc::new(manual);
    assert!(
        immediate.persists_durably(),
        "Immediate-mode SQLite must pass the invitation durability gate"
    );
    assert!(
        manual.persists_durably(),
        "Manual-mode SQLite must pass the invitation durability gate \
         (flush writes through in one transaction)"
    );

    let stub: Arc<dyn PlatformWalletPersistence> = Arc::new(StoreOnlyPersister);
    assert!(
        !stub.persists_durably(),
        "a backend that does not attest durability must stay fail-closed"
    );
}
