#![allow(clippy::field_reassign_with_default)]

//! TC-CODE-003 — `PlatformWalletPersistence::delete_wallet` is
//! reachable through the trait (not just the inherent method on
//! `SqlitePersister`). Dispatch happens through
//! `Arc<dyn PlatformWalletPersistence>` so consumers don't need a
//! concrete backend type at the call site.
//!
//! - TC-CODE-003-default — trait default `delete_wallet` returns an
//!   empty report (proven via a NoPlatformPersistence-style stub).
//! - TC-CODE-003-sqlite — trait-dispatched `delete_wallet` on
//!   `SqlitePersister` actually cascades the on-disk rows.

mod common;

use std::sync::Arc;

use common::{ensure_wallet_meta, fresh_persister, ro_conn, wid};
use platform_wallet::changeset::{
    ClientStartState, CoreChangeSet, DeleteWalletReport, PersistenceError, PlatformWalletChangeSet,
    PlatformWalletPersistence,
};
use platform_wallet::wallet::platform_wallet::WalletId;

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

/// Stub persister that exercises the `delete_wallet` trait default —
/// the empty impl below inherits it.
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
    assert!(report.rows_removed_per_table.is_empty());
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
