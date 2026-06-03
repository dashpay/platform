#![allow(clippy::field_reassign_with_default)]

//! TC-CODE-003 / TC-CODE-026 — `PlatformWalletPersistence::delete_wallet`
//! and `::commit_writes` are reachable through the trait (not just the
//! inherent methods on `SqlitePersister`). Dispatch happens through
//! `Arc<dyn PlatformWalletPersistence>` so consumers don't need a
//! concrete backend type at the call site.
//!
//! - TC-CODE-003-default — trait default `delete_wallet` returns an
//!   empty report (proven via a NoPlatformPersistence-style stub).
//! - TC-CODE-003-sqlite — trait-dispatched `delete_wallet` on
//!   `SqlitePersister` actually cascades the on-disk rows.
//! - TC-CODE-026-1 — trait default `commit_writes` returns an empty
//!   report (same stub backend).
//! - TC-CODE-026-2 — trait-dispatched `commit_writes` on
//!   `SqlitePersister` matches the inherent behavior (success).

mod common;

use std::sync::Arc;

use common::{ensure_wallet_meta, fresh_persister, fresh_persister_with_mode, ro_conn, wid};
use platform_wallet::changeset::{
    ClientStartState, CommitReport, CoreChangeSet, DeleteWalletReport, PersistenceError,
    PlatformWalletChangeSet, PlatformWalletPersistence,
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

/// Stub persister that exercises every trait default — `delete_wallet`
/// and `commit_writes` are inherited from the trait, so an empty impl
/// suffices.
struct DefaultsOnlyPersister;

impl PlatformWalletPersistence for DefaultsOnlyPersister {
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

/// TC-CODE-003-default — `delete_wallet` default impl returns an
/// empty report keyed by the requested wallet id. Backends with no
/// per-wallet disk state inherit this; consumers use the same Ok-arm
/// regardless of backend.
#[test]
fn tc_code_003_default_delete_wallet_returns_empty_report() {
    let persister: Arc<dyn PlatformWalletPersistence> = Arc::new(DefaultsOnlyPersister);
    let wallet_id = wid(0xAB);
    let report: DeleteWalletReport = persister
        .delete_wallet(wallet_id)
        .expect("default delete_wallet must be infallible");
    assert_eq!(report.wallet_id, wallet_id);
    assert!(report.backup_path.is_none());
}

/// TC-CODE-003-sqlite — trait-dispatched `delete_wallet` on
/// `SqlitePersister` cascades the on-disk rows. Without the trait
/// impl this call would resolve to the default and silently leave
/// the rows in place.
#[test]
fn tc_code_003_sqlite_trait_delete_wallet_cascades_rows() {
    let (persister, _tmp, path) = fresh_persister();
    let w = wid(0x55);
    ensure_wallet_meta(&persister, &w);
    // Land a per-wallet row via the trait so we have something to
    // cascade.
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

    // Dispatch through the trait — this is the call shape
    // `PlatformWalletManager` uses.
    let report = PlatformWalletPersistence::delete_wallet(&persister, w)
        .expect("trait delete_wallet must succeed");
    assert_eq!(report.wallet_id, w);
    assert!(
        report.backup_path.is_some(),
        "trait-dispatched delete_wallet must take an auto-backup (safe-by-default)"
    );

    assert_eq!(count_for(&w), 0);
}

/// TC-CODE-026-1 — `commit_writes` default impl returns an empty
/// `CommitReport`. Drives backwards-compat for stubs +
/// `NoPlatformPersistence`-style implementors that don't track dirty
/// state.
#[test]
fn tc_code_026_1_commit_writes_default_returns_empty_report() {
    let persister: Arc<dyn PlatformWalletPersistence> = Arc::new(DefaultsOnlyPersister);
    let report: CommitReport = persister
        .commit_writes()
        .expect("default commit_writes must be infallible");
    assert!(report.is_ok());
    assert!(report.succeeded.is_empty());
    assert!(report.failed.is_empty());
    assert!(report.still_pending.is_empty());
}

/// TC-CODE-026-2 — trait-dispatched `commit_writes` on
/// `SqlitePersister` flushes every dirty wallet just like the
/// inherent method (no behavioral drift across dispatch).
#[test]
fn tc_code_026_2_sqlite_trait_commit_writes_flushes_dirty() {
    let (persister, _tmp, path) = fresh_persister_with_mode(FlushMode::Manual);
    let a = wid(0x11);
    let b = wid(0x22);
    ensure_wallet_meta(&persister, &a);
    ensure_wallet_meta(&persister, &b);
    PlatformWalletPersistence::store(&persister, a, changeset(core_with_height(3, 3)))
        .expect("store A");
    PlatformWalletPersistence::store(&persister, b, changeset(core_with_height(4, 4)))
        .expect("store B");

    let report = PlatformWalletPersistence::commit_writes(&persister)
        .expect("trait commit_writes must succeed");
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
